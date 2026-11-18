use std::path::PathBuf;

use requestor_core::settings::ClientCertificate;
use requestor_core::workspace::Workspace;

use super::state_with;
use crate::state::{Action, CertificateFile, CertificatePart, Effect};

#[test]
fn a_chosen_file_fills_the_certificate_it_was_chosen_for() {
    let mut state = state_with(Workspace::default());
    state
        .workspace
        .settings
        .request
        .client_certificates
        .push(ClientCertificate::default());
    let key = CertificateFile {
        index: 0,
        part: CertificatePart::Key,
    };
    let effects = state.apply(Action::ChooseCertificateFile(key), 0.0);
    assert!(matches!(effects.as_slice(), [Effect::PickCertificateFile(file)] if *file == key));

    state.apply(
        Action::CertificateFilePicked {
            file: key,
            path: PathBuf::from("/certs/client.key"),
        },
        0.0,
    );
    state.apply(
        Action::CertificateFilePicked {
            file: CertificateFile {
                index: 0,
                part: CertificatePart::Certificate,
            },
            path: PathBuf::from("/certs/client.pem"),
        },
        0.0,
    );
    state.apply(
        Action::CertificateFilePicked {
            file: CertificateFile { index: 5, ..key },
            path: PathBuf::from("/elsewhere.key"),
        },
        0.0,
    );
    let certificates = &state.workspace.settings.request.client_certificates;
    assert_eq!(certificates.len(), 1);
    assert_eq!(
        certificates[0].certificate_path,
        PathBuf::from("/certs/client.pem")
    );
    assert_eq!(
        certificates[0].key_path,
        Some(PathBuf::from("/certs/client.key"))
    );
}
