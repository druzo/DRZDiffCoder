use crate::diff_vm::DiffViewModel;
use crate::editor_vm::EditorViewModel;
use std::path::Path;

pub struct AppViewModel {
    diff: Option<DiffViewModel>,
    error: Option<String>,
}

impl AppViewModel {
    pub fn empty() -> AppViewModel {
        AppViewModel {
            diff: None,
            error: None,
        }
    }

    pub fn open_pair(left: &Path, right: &Path) -> AppViewModel {
        let mut vm = AppViewModel::empty();
        vm.open_pair_command(left, right);
        vm
    }

    pub fn open_pair_command(&mut self, left: &Path, right: &Path) {
        let result = (|| -> Result<DiffViewModel, String> {
            let l = EditorViewModel::open(left).map_err(|e| format!("{}: {e}", left.display()))?;
            let r =
                EditorViewModel::open(right).map_err(|e| format!("{}: {e}", right.display()))?;
            let mut d = DiffViewModel::new(l, r);
            d.flush_diff_now();
            Ok(d)
        })();
        match result {
            Ok(d) => {
                self.diff = Some(d);
                self.error = None;
            }
            Err(msg) => {
                self.diff = None;
                self.error = Some(format!("open failed: {msg}"));
            }
        }
    }

    pub fn diff(&self) -> Option<&DiffViewModel> {
        self.diff.as_ref()
    }
    pub fn diff_mut(&mut self) -> Option<&mut DiffViewModel> {
        self.diff.as_mut()
    }

    pub fn save_all(&mut self) {
        if let Some(d) = &mut self.diff {
            let save_side = |side: &mut EditorViewModel| -> Result<(), String> {
                if side.is_dirty() {
                    side.save().map_err(|e| format!("save failed: {e}"))?;
                }
                Ok(())
            };
            let result = save_side(d.left_mut()).and_then(|()| save_side(d.right_mut()));
            if let Err(msg) = result {
                self.error = Some(msg);
            }
        }
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn dismiss_error(&mut self) {
        self.error = None;
    }

    pub fn title(&self) -> String {
        match &self.diff {
            Some(d) => {
                let name = |vm: &EditorViewModel| -> String {
                    vm.path()
                        .and_then(|p: &Path| p.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "(untitled)".into())
                };
                let dirty = if d.left().is_dirty() || d.right().is_dirty() {
                    " *"
                } else {
                    ""
                };
                format!(
                    "DRZDiffCoder — {} ↔ {}{}",
                    name(d.left()),
                    name(d.right()),
                    dirty
                )
            }
            None => "DRZDiffCoder".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff_vm::MergeDirection;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    fn tmpfile(content: &str, name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("drzvm_test");
        std::fs::create_dir_all(&dir).ok();
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn open_pair_success() {
        let l = tmpfile("a\n", "vm_l.txt");
        let r = tmpfile("b\n", "vm_r.txt");
        let mut vm = AppViewModel::open_pair(&l, &r);
        assert!(vm.diff().is_some());
        assert!(vm.error().is_none());
        vm.diff_mut().unwrap().flush_diff_now();
        assert_eq!(vm.diff().unwrap().hunks().len(), 1);
    }

    #[test]
    fn open_pair_missing_file_sets_error() {
        let r = tmpfile("b\n", "vm_r2.txt");
        let vm = AppViewModel::open_pair(Path::new("/nonexistent/x.txt"), &r);
        assert!(vm.diff().is_none());
        assert!(vm.error().is_some());
    }

    #[test]
    fn save_all_clears_dirty() {
        let l = tmpfile("a\n", "vm_l3.txt");
        let r = tmpfile("b\n", "vm_r3.txt");
        let mut vm = AppViewModel::open_pair(&l, &r);
        let d = vm.diff_mut().unwrap();
        d.flush_diff_now();
        d.merge_chunk(0, MergeDirection::LeftToRight);
        vm.save_all();
        assert!(!vm.diff().unwrap().right().is_dirty());
    }

    #[test]
    fn title_shows_dirty_marker() {
        let l = tmpfile("a\n", "vm_l4.txt");
        let r = tmpfile("b\n", "vm_r4.txt");
        let mut vm = AppViewModel::open_pair(&l, &r);
        assert!(!vm.title().contains('*'));
        vm.diff_mut().unwrap().right_mut().edit(0, 0, "z");
        assert!(vm.title().contains('*'));
    }
}
