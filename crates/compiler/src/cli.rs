use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use compiler::db::RootDatabase;
use compiler::project::setup_project;
use defs::db::DefsGroup;
use defs::ids::ModuleId;
use filesystem::db::FilesGroup;
use filesystem::ids::FileLongId;
use lowering::db::LoweringGroup;
use parser::db::ParserGroup;
use semantic::db::SemanticGroup;
use sierra_generator::db::SierraGenGroup;
use sierra_generator::replace_ids::replace_sierra_ids_in_program;

/// Command line args parser.
/// Exits with 0/1 if the input is formatted correctly/incorrectly.
#[derive(Parser, Debug)]
#[clap(version, verbatim_doc_comment)]
struct Args {
    /// The file to compile
    path: String,
    /// The output file name (default: stdout).
    output: Option<String>,
    /// Replaces sierra ids with human readable ones.
    #[arg(short, long, default_value_t = false)]
    replace_ids: bool,
}

/// Prints the diagnostics to stderr.
fn print_diagnostics(db: &mut RootDatabase) {
    for crate_id in db.crates() {
        for module_id in &*db.crate_modules(crate_id) {
            if let Some(file_id) = db.module_file(*module_id) {
                if db.file_content(file_id).is_none() {
                    if let ModuleId::CrateRoot(_) = *module_id {
                        match db.lookup_intern_file(file_id) {
                            FileLongId::OnDisk(path) => eprintln!("{} not found", path.display()),
                            FileLongId::Virtual(_) => panic!("Missing virtual file."),
                        }
                    }
                } else {
                    eprint!("{}", db.file_syntax_diagnostics(file_id).format(db));
                }

                if let Some(diag) = db.module_semantic_diagnostics(*module_id) {
                    eprint!("{}", diag.format(db));
                }

                if let Some(diag) = db.module_lowering_diagnostics(*module_id) {
                    eprint!("{}", diag.format(db));
                }

                eprint!("{}", db.module_sierra_diagnostics(*module_id).format(db));
            }
        }
    }
}

fn main() -> ExitCode {
    let args = Args::parse();

    let mut db_val = RootDatabase::default();
    let db = &mut db_val;

    if let Err(error) = setup_project(db, Path::new(&args.path)) {
        eprintln!("{}", error);
        return ExitCode::FAILURE;
    }

    let Some(mut sierra_program) = db.get_sierra_program() else {
        print_diagnostics(db);
        return ExitCode::FAILURE;
    };

    if args.replace_ids {
        sierra_program = Arc::new(replace_sierra_ids_in_program(db, &sierra_program));
    }

    match args.output {
        Some(path) => {
            fs::write(path, format!("{}", sierra_program)).expect("Failed to write output.")
        }
        None => println!("{}", sierra_program),
    }

    ExitCode::SUCCESS
}