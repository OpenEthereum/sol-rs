// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

#[cfg(not(windows))]
mod platform {
    use std::process::Command;

    pub fn solc() -> Command {
        Command::new("solc")
    }
}

#[cfg(windows)]
mod platform {
    use std::process::Command;

    pub fn solc() -> Command {
        let command = Command::new("cmd.exe");
        command.arg("/c").arg("solc.cmd");
        command
    }
}

use std::path::Path;
use std::{fs, io};
use std::process::Stdio;

/// Compiles all solidity files in given directory.
pub fn compile<T: AsRef<Path>>(path: T) {
    let filename = fs::canonicalize(&path)
        .unwrap_or_else(|e| panic!("Error canonicalizing the contract path: {}", e));

    let mut command = platform::solc();
    command
        // Output contract binary
		.arg("--bin")
        // Output contract abi
		.arg("--abi")
        // Overwrite existing output files (*.abi, *.bin, etc.)
		.arg("--overwrite")
        // Compile optimized evm-bytecode
        .arg("--optimize")
        // Create one file per component
        .arg("-o")
        .arg(filename);

    for file in sol_files(&path).expect("Contracts directory is not readable.") {
        command.arg(file);
    }

    let child = command
        .current_dir(path)
        .status()
        .unwrap_or_else(|e| panic!("Error compiling solidity contracts: {}", e));
    assert!(
        child.success(),
        "There was an error while compiling contracts code."
    );
}

/// Link libraries to given bytecode.
pub fn link<T: AsRef<Path>>(libraries: Vec<String>, target: String, path: T) {
    let mut command = platform::solc();
    command
        // Link mode
        .arg("--link");

    for library in libraries {
        command
            .arg("--libraries")
            .arg(library);
    }

    command.arg(target);

    let child = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(path)
        .status()
        .unwrap_or_else(|e| panic!("Error linking solidity contracts: {}", e));

    assert!(
        child.success(),
        "There was an error while linking contracts code."
    );
}

fn sol_files<T: AsRef<Path>>(path: T) -> io::Result<Vec<String>> {
    let mut sol_files = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let filename = path.file_name()
            .and_then(|os_str| os_str.to_str().to_owned());
        match filename {
            Some(file) if file.ends_with(".sol") => {
                sol_files.push(file.into());
            }
            _ => {}
        }
    }

    Ok(sol_files)
}
