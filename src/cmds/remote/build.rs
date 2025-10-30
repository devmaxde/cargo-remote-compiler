use std::process::exit;

use log::error;

use crate::{core::remote_exec, ExecOpts};

pub fn cmd_build(exec: ExecOpts, options: Vec<String>) {
    if let Err(e) = remote_exec(exec, "build", options) {
        error!("{}", e);
        exit(4)
    }
}
