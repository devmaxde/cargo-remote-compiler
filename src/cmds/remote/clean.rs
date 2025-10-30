use std::process::exit;

use log::error;

use crate::{core::remote_exec, ExecOpts};

pub fn cmd_clean(exec: ExecOpts, options: Vec<String>) {
    if let Err(e) = remote_exec(exec, "clean", options) {
        error!("{}", e);
        exit(4)
    }
}
