use std::io;
use std::path::PathBuf;

use requestor_core::workspace::DataFolder;

use super::AppState;

const MOVED_TEXT: &str = "Requestor will use this data folder after a restart:";
const RESET_TEXT: &str = "Requestor will use its own data folder again after a restart";
const MOVE_FAILED: &str = "Could not change the data folder:";
const COPY_FAILED: &str =
    "Could not copy your data to the new data folder, so the previous one is still in use. Requestor tries again at the next start:";
const UNAVAILABLE_TEXT: &str = "The data folder is not available, so Requestor opened its default folder. Changes made now stay there:";

impl AppState {
    pub fn with_data_folder(mut self, folder: DataFolder, settled: io::Result<()>) -> Self {
        if let Err(error) = settled {
            self.warn(format!("{COPY_FAILED} {error}"));
        }
        if let Some(missing) = folder.unavailable() {
            self.warn(format!("{UNAVAILABLE_TEXT} {}", missing.display()));
        }
        self.data_folder = Some(folder);
        self
    }

    pub(super) fn move_data_folder(&mut self, path: PathBuf) {
        let Some(folder) = &self.data_folder else {
            return;
        };
        match folder.move_to(&path) {
            Ok(()) => self.inform(format!("{MOVED_TEXT} {}", folder.next_location().display())),
            Err(error) => self.report_error(format!("{MOVE_FAILED} {error}")),
        }
    }

    pub(super) fn reset_data_folder(&mut self) {
        let Some(folder) = &self.data_folder else {
            return;
        };
        match folder.reset() {
            Ok(()) => self.inform(RESET_TEXT),
            Err(error) => self.report_error(format!("{MOVE_FAILED} {error}")),
        }
    }
}
