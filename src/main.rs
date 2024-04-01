/* SPDX-License-Identifier: GPL-3.0-or-later */

mod handlers;
mod options;
mod simplelog;

use anyhow::{Result, anyhow};
use log::{debug, warn};
use std::env;
use std::path::Path;
use std::rc::Rc;

fn brp_check(config: &options::Config) -> Result<()> {
    // env::current_exe() does readlink("/proc/self/exe"), which returns
    // the target binary, so we cannot use that.

    let arg0 = env::args().next().unwrap();

    let brp = config.brp || Path::new(&arg0)
        .file_name()
        .ok_or(anyhow!("Exe path doesn't have a file name?"))?
        .to_str()
        .ok_or(anyhow!("Exe file name is not valid unicode"))?
        .starts_with("brp-");

    debug!("Running as {}… (brp={})", arg0, if brp { "true" } else { "false" });

    if brp {
        let build_root = env::var("RPM_BUILD_ROOT")
            .map_err(|_| anyhow!("RPM_BUILD_ROOT variable is not defined"))?;

        if build_root.is_empty() {
            return Err(anyhow!("Empty RPM_BUILD_ROOT is not allowed"));
        }

        let build_root_path = Path::new(&build_root).canonicalize()
            .map_err(|e| anyhow!("Cannot canonicalize RPM_BUILD_ROOT={:?}: {}", build_root, e))?;

        if build_root_path == Path::new("/") {
            return Err(anyhow!("RPM_BUILD_ROOT={:?} is not allowed", build_root));
        }

        for arg in &config.args {
            if !arg.starts_with(&build_root_path) {
                return Err(anyhow!("Path {:?} is outside of $RPM_BUILD_ROOT", arg));
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let config = match options::Config::make()? {
        None => { return Ok(()); },
        Some(some) => some
    };

    brp_check(&config)?;

    let config = Rc::new(config);
    let handlers = handlers::make_handlers(&config);

    let mut inodes_seen = handlers::inodes_seen();

    for input_path in &config.args {
        handlers::process_file_or_dir(&handlers, &mut inodes_seen, input_path).unwrap_or_else(|err| {
            warn!("{}: failed to process: {}", input_path.display(), err);
            0
        });
    }

    Ok(())
}
