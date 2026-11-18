use std::path::PathBuf;

use requestor_core::settings::ClientCertificate;

use super::{Action, AppState, Effect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificatePart {
    Certificate,
    Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificateFile {
    pub index: usize,
    pub part: CertificatePart,
}

impl CertificateFile {
    fn apply(self, certificate: &mut ClientCertificate, path: PathBuf) {
        match self.part {
            CertificatePart::Certificate => certificate.certificate_path = path,
            CertificatePart::Key => certificate.key_path = Some(path),
        }
    }
}

impl AppState {
    pub(super) fn dispatch_certificate_action(
        &mut self,
        action: Action,
        effects: &mut Vec<Effect>,
    ) {
        match action {
            Action::ChooseCertificateFile(file) => effects.push(Effect::PickCertificateFile(file)),
            Action::CertificateFilePicked { file, path } => {
                self.certificate_file_picked(file, path)
            }
            _ => {}
        }
    }

    fn certificate_file_picked(&mut self, file: CertificateFile, path: PathBuf) {
        let certificates = &mut self.workspace.settings.request.client_certificates;
        if let Some(certificate) = certificates.get_mut(file.index) {
            file.apply(certificate, path);
            self.unsaved.touch_settings(self.now);
        }
    }
}
