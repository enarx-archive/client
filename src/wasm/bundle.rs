use crate::util::ofile::OutputFile;

use std::convert::TryFrom;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use anyhow::Result;
use codicon::*;
use lebicon::Leb128;
use structopt::StructOpt;
use tar::{Archive, Builder};
use wasmparser::{Chunk, Parser, Payload};

#[derive(StructOpt, Debug)]
pub struct Bundle {
    /// The files you want to bundle into a wasm binary
    files: PathBuf,

    /// The input wasm binary
    iwasm: PathBuf,

    /// The output wasm binary
    owasm: PathBuf,

    #[structopt(short, long, default_value = ".enarx.resources")]
    section: String,
}

/// Write a tarball containing all the files under the input directory.
fn mktar(input: &Path, output: &mut File) -> Result<()> {
    let mut tar = Builder::new(output);

    for entry in walkdir::WalkDir::new(input) {
        let entry = entry?;

        let path = entry.path();
        if path == input {
            continue;
        }

        let rel = path.strip_prefix(input)?;
        tar.append_path_with_name(path, rel)?;
    }

    tar.finish()?;
    Ok(())
}

/// This function prepares our input tarball.
///
/// If `files` points to a directory, we tar it up.
/// If `files` points to a tarball, we open it.
///
/// Note that the position of the returned `File` is undefined.
fn prepare_tarball(ifile: &Path) -> Result<File> {
    Ok(if ifile.metadata()?.is_dir() {
        let mut tar = tempfile::tempfile()?;
        mktar(ifile, &mut tar)?;
        tar
    } else {
        let tar = File::open(ifile)?;
        Archive::new(&tar).entries()?;
        tar
    })
}

/// Copies the wasm from `ifile` to `ofile`, but drops the `section` along the
/// way. Returns the `OutputFile` to allow for further appending.
fn strip_section<T, U>(section: &str, ifile: T, ofile: U) -> Result<OutputFile<U>>
where
    T: AsRef<Path>,
    U: AsRef<Path>,
{
    let mut buffer = Vec::new();
    let mut parser = Parser::new(0);
    let mut eofile = false;
    let mut pstack = Vec::new();
    let mut inwasm = File::open(ifile)?;
    let mut output = OutputFile::create(ofile)?;

    loop {
        let (consumed, payload) = match parser.parse(&buffer, eofile)? {
            Chunk::Parsed { consumed, payload } => (consumed, payload),
            Chunk::NeedMoreData(hint) => {
                assert!(!eofile);

                let len = buffer.len();
                buffer.extend((0..hint).map(|_| 0u8));

                let n = inwasm.read(&mut buffer[len..])?;
                buffer.truncate(len + n);

                eofile = n == 0;
                continue;
            }
        };

        match payload {
            Payload::ModuleCodeSectionEntry { parser: sp, .. } => {
                pstack.push(parser);
                parser = sp;
            }

            Payload::End => {
                if let Some(p) = pstack.pop() {
                    parser = p;
                } else {
                    return Ok(output);
                }
            }

            Payload::CustomSection { name, .. } if name == section => {}

            _ => {
                output.write_all(&buffer[..consumed])?;
            }
        }

        buffer.drain(..consumed);
    }
}

impl crate::Command for Bundle {
    fn execute(self) -> anyhow::Result<()> {
        // Encode the name length.
        let name = self.section.as_bytes();
        let mut name_len = Vec::new();
        name.len().encode(&mut name_len, Leb128)?;

        // Get the tarball and its size.
        let mut tarball = prepare_tarball(&self.files)?;
        let tarball_len = tarball.seek(SeekFrom::End(0))?;
        tarball.seek(SeekFrom::Start(0))?;

        // Calculate the length of the custom section payload.
        let payload_len = usize::try_from(tarball_len)? + name.len() + name_len.len();

        // Strip the section from the existing wasm file.
        let mut output = strip_section(&self.section, self.iwasm, self.owasm)?;

        // Write out the custom section.
        output.write_all(&[0])?; // section id == 0 (custom)
        payload_len.encode(&mut output, Leb128)?;
        output.write_all(&name_len)?;
        output.write_all(name)?;
        std::io::copy(&mut tarball, &mut output)?;

        output.done();
        Ok(())
    }
}
