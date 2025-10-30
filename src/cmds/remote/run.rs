use std::process::exit;

use log::error;

use crate::{core::remote_exec, ExecOpts};

pub fn cmd_run(exec: ExecOpts, options: Vec<String>) {
    if let Err(e) = remote_exec(exec, "run", options) {
        error!("{}", e);
        exit(4)
    }
}
