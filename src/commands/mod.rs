// SPDX-License-Identifier: Apache-2.0

//! CLI command implementations extracted from main.rs.
//!
//! Each submodule contains a `run()` function that receives the
//! dependencies it needs (ops, db, etc.) and performs the command.

pub mod activate;
pub mod add;
pub mod clone;
pub mod config;
pub mod create;
pub mod diff;
pub mod export;
pub mod find;
pub mod health;
pub mod import;
pub mod info;
pub mod inspect;
pub mod install;
pub mod label;
pub mod link;
pub mod list;
pub mod log;
pub mod note;
pub mod rename;
pub mod reset;
pub mod rm;
pub mod run;
pub mod setup;
pub mod status;
pub mod template;
pub mod uninstall;
