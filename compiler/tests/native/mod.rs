pub(crate) mod alias_restarts;
pub(crate) mod alias_writes;
pub(crate) mod allocator_bounds;
pub(crate) mod allocator_carriers;
pub(crate) mod binding_fields;
pub(crate) mod boolean_blocks;
pub(crate) mod boolean_equality;
pub(crate) mod boolean_fields;
pub(crate) mod boolean_modules;
pub(crate) mod boolean_primaries;
pub(crate) mod borrow_syntax;
pub(crate) mod carried_borrows;
pub(crate) mod carried_list_borrows;
pub(crate) mod carried_lists;
pub(crate) mod carried_record_borrows;
pub(crate) mod carried_records;
pub(crate) mod carried_scalars;
pub(crate) mod carried_writes;
pub(crate) mod changing_published;
pub(crate) mod cli;
pub(crate) mod composed_inputs;
pub(crate) mod computed_fields;
pub(crate) mod computed_types;
pub(crate) mod conditional_inputs;
pub(crate) mod conditional_types;
pub(crate) mod control;
pub(crate) mod discarded_aliases;
pub(crate) mod dispatch;
pub(crate) mod doc_fences;
pub(crate) mod documentation;
pub(crate) mod documented_modules;
pub(crate) mod dynamic_lists;
pub(crate) mod effectful_lists;
pub(crate) mod element_borrows;
pub(crate) mod element_writes;
pub(crate) mod emitted_borrows;
pub(crate) mod emitted_slots;
pub(crate) mod examples;
pub(crate) mod exclusive_carried;
pub(crate) mod exclusive_carried_elements;
pub(crate) mod exclusive_carried_records;
pub(crate) mod exclusive_elements;
pub(crate) mod exclusive_fields;
pub(crate) mod exclusive_functions;
pub(crate) mod exclusive_indexed_fields;
pub(crate) mod exclusive_nested_elements;
pub(crate) mod exclusive_projected_elements;
pub(crate) mod exclusive_references;
pub(crate) mod exclusive_slot_fields;
pub(crate) mod exclusive_slots;
pub(crate) mod expired_restarts;
pub(crate) mod exported_calls;
pub(crate) mod exported_types;
pub(crate) mod fields;
pub(crate) mod file_modules;
pub(crate) mod fixed_published;
pub(crate) mod function_borrows;
pub(crate) mod guarded_references;
pub(crate) mod header_activity;
pub(crate) mod immutable_slots;
pub(crate) mod imported_inputs;
pub(crate) mod integer_blocks;
pub(crate) mod late_published;
pub(crate) mod leave_references;
pub(crate) mod list_contexts;
pub(crate) mod lists;
pub(crate) mod mixed_headers;
pub(crate) mod mixed_primary_inputs;
pub(crate) mod mixed_writes;
pub(crate) mod mutable_carriers;
pub(crate) mod mutable_references;
pub(crate) mod nested_writes;
pub(crate) mod panic_sites;
pub(crate) mod panics;
pub(crate) mod primary_inputs;
pub(crate) mod published_snapshots;
pub(crate) mod reborrows;
pub(crate) mod record_scratch;
pub(crate) mod reference_blocks;
pub(crate) mod reference_fields;
pub(crate) mod reference_records;
pub(crate) mod reference_returns;
pub(crate) mod reference_slots;
pub(crate) mod reference_temporaries;
pub(crate) mod reference_unions;
pub(crate) mod references;
pub(crate) mod required_boolean_ops;
pub(crate) mod required_booleans;
pub(crate) mod required_comparisons;
pub(crate) mod required_records;
pub(crate) mod required_scalar_blocks;
pub(crate) mod restart_references;
pub(crate) mod scalars;
pub(crate) mod scoped_imports;
pub(crate) mod tagged_allocators;
pub(crate) mod temporary_borrows;
pub(crate) mod transitive_borrows;
pub(crate) mod transitive_restarts;
pub(crate) mod type_block_equality;
pub(crate) mod type_equality;
pub(crate) mod type_exports;
pub(crate) mod type_subtraction;
pub(crate) mod union_aliases;
pub(crate) mod unions;
pub(crate) mod widened_aliases;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) static NEXT: AtomicU64 = AtomicU64::new(0);

pub(crate) struct Case {
    pub(crate) path: PathBuf,
    pub(crate) source: PathBuf,
}

impl Case {
    pub(crate) fn new(source: &str) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("meowy-test-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        let entry = path.join("main.mwy");
        fs::write(&entry, source).unwrap();
        Self {
            path,
            source: entry,
        }
    }

    pub(crate) fn command(&self, action: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_meowy"))
            .arg(action)
            .arg(&self.source)
            .args(["--standalone", "--quiet", "--color", "never"])
            .args(args)
            .current_dir(&self.path)
            .output()
            .unwrap()
    }

    pub(crate) fn runs(&self, expected: &[u8]) {
        for profile in ["debug", "release"] {
            let result = self.command("run", &["--profile", profile]);
            assert!(
                result.status.success(),
                "{profile}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.stdout, expected, "{profile}");
            assert!(
                result.stderr.is_empty(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
