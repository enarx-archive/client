mod util;
mod wasm;

use structopt::StructOpt;

trait Command {
    fn execute(self) -> anyhow::Result<()>;
}

#[derive(StructOpt, Debug)]
enum Top {
    /// WebAssembly utilities
    Wasm(wasm::Wasm),
}

impl crate::Command for Top {
    fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Wasm(w) => w.execute(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    Top::from_args().execute()
}
