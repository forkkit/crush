use crate::lang::errors::{error, CrushResult};
use crate::lang::execution_context::CommandContext;
use crate::lang::data::table::{Row, ColumnVec};
use signature::signature;
use crate::lang::command::OutputType::Unknown;
use crate::lang::value::Field;
use std::collections::HashSet;

#[signature(
drop,
can_block = true,
short = "Drop all fields mentioned from input, copy remainder of input",
long = "This command is does the opposite of the select command.\n    It copies all column except the ones specified from input to output.",
example= "ps | drop ^vms ^rss # Drop memory usage columns from output of ps",
output = Unknown,
)]
pub struct Drop {
    #[unnamed()]
    drop: Vec<Field>,
}

fn drop(context: CommandContext) -> CrushResult<()> {
    let cfg: Drop = Drop::parse(context.arguments.clone(), &context.printer)?;
    match context.input.recv()?.stream() {
        Some(mut input) => {
            let t = input.types();
            let drop = cfg.drop.iter()
                .map(|f| t.find(f))
                .collect::<CrushResult<HashSet<usize>>>()?;
            let inc: Vec<bool> = (0..t.len()).into_iter().map(|idx| drop.contains(&idx)).collect();
            let mut it = inc.iter();
            let output = context.output.initialize(t.to_vec().drain(..).filter(|_| !*(it.next().unwrap())).collect())?;
            while let Ok(row) = input.read() {
                let mut row = row.into_vec();
                let mut it = inc.iter();
                output.send(
                    Row::new(
                    row.drain(..).filter(|_| !*(it.next().unwrap())).collect()
                    )
                )?;
            }
            Ok(())
        }
        None => error("Expected a stream"),
    }
}