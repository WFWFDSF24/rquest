pub mod compression;
#[cfg(any(feature = "webpki-roots", feature = "native-roots"))]
mod load;

use super::{sv_handler, TlsResult};
use boring2::{ssl::SslConnectorBuilder, x509::store::X509Store};
use boring_sys2 as ffi;
use foreign_types::ForeignTypeRef;

/// The root certificate store.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub enum RootCertStore {
    /// An owned `X509Store`.
    Owned(X509Store),

    /// A borrowed `X509Store`.
    Borrowed(&'static X509Store),

    /// Use the system's native certificate store.
    #[default]
    Default,
}

/// ====== impl RootCertsStore ======
impl RootCertStore {
    /// Applies the root certificate store to the TLS builder.
    pub(crate) fn apply(self, builder: &mut SslConnectorBuilder) -> TlsResult<()> {
        // Conditionally configure the TLS builder based on the "native-roots" feature.
        // If no custom CA cert store, use the system's native certificate store if the feature is enabled.
        match self {
            RootCertStore::Default => {
                // WebPKI root certificates are enabled (regardless of whether native-roots is also enabled).
                #[cfg(any(feature = "webpki-roots", feature = "native-roots"))]
                {
                    if let Some(cert_store) = load::LOAD_CERTS.as_deref() {
                        log::debug!("Using CA certs from webpki/native roots");
                        sv_handler(unsafe {
                            ffi::SSL_CTX_set1_verify_cert_store(
                                builder.as_ptr(),
                                cert_store.as_ptr(),
                            )
                        })?;
                    } else {
                        log::debug!("No CA certs provided, using system default");
                        builder.set_default_verify_paths()?;
                    }
                }

                // Neither native-roots nor WebPKI roots are enabled, proceed with the default builder.
                #[cfg(not(any(feature = "webpki-roots", feature = "native-roots")))]
                {
                    builder.set_default_verify_paths()?;
                }
            }
            RootCertStore::Owned(cert_store) => {
                builder.set_verify_cert_store(cert_store)?;
            }
            RootCertStore::Borrowed(cert_store) => {
                sv_handler(unsafe {
                    ffi::SSL_CTX_set1_verify_cert_store(builder.as_ptr(), cert_store.as_ptr())
                })?;
            }
        }

        Ok(())
    }
}

macro_rules! impl_root_cert_store {
    ($($type:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$type> for RootCertStore {
                fn from(store: $type) -> Self {
                    Self::$variant(store)
                }
            }
        )*
    };

    ($($type:ty => $variant:ident, $unwrap:expr),* $(,)?) => {
        $(
            impl From<$type> for RootCertStore {
                fn from(store: $type) -> Self {
                    $unwrap(store).map(Self::$variant).unwrap_or_default()
                }
            }
        )*
    };
}

impl_root_cert_store!(
    X509Store => Owned,
    &'static X509Store => Borrowed,
);

impl_root_cert_store!(
    Option<X509Store> => Owned, |s| s,
    Option<&'static X509Store> => Borrowed, |s| s,
);

impl<F> From<F> for RootCertStore
where
    F: Fn() -> Option<&'static X509Store>,
{
    fn from(func: F) -> Self {
        func().map(Self::Borrowed).unwrap_or_default()
    }
}
