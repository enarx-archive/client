mod bundle;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Wasm {
    /// Embed file resources
    Bundle(bundle::Bundle),
}

impl crate::Command for Wasm {
    fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Bundle(b) => b.execute(),
        }
    }
}
