use http::{header, HeaderMap, HeaderName, HeaderValue};
use rquest::{
    join, AlpnProtos, AlpsProtos, CertCompressionAlgorithm, ExtensionType, SslCurve, TlsSettings,
    TlsVersion,
};
use rquest::{Client, ImpersonateSettings};
use rquest::{Http2Settings, PseudoOrder::*, SettingsOrder::*};
use rquest::{Priority, StreamDependency, StreamId};

// ============== TLS Extension Algorithms ==============

const CURVES: &[SslCurve] = &[
    SslCurve::X25519,
    SslCurve::SECP256R1,
    SslCurve::SECP384R1,
    SslCurve::SECP521R1,
    SslCurve::FFDHE2048,
    SslCurve::FFDHE3072,
];

const CIPHER_LIST: &str = join!(
    ":",
    "TLS_AES_128_GCM_SHA256",
    "TLS_CHACHA20_POLY1305_SHA256",
    "TLS_AES_256_GCM_SHA384",
    "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
    "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256",
    "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256",
    "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256",
    "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
    "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384",
    "TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA",
    "TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA",
    "TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA",
    "TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA",
    "TLS_RSA_WITH_AES_128_GCM_SHA256",
    "TLS_RSA_WITH_AES_256_GCM_SHA384",
    "TLS_RSA_WITH_AES_128_CBC_SHA",
    "TLS_RSA_WITH_AES_256_CBC_SHA"
);

const SIGALGS_LIST: &str = join!(
    ":",
    "ecdsa_secp256r1_sha256",
    "ecdsa_secp384r1_sha384",
    "ecdsa_secp521r1_sha512",
    "rsa_pss_rsae_sha256",
    "rsa_pss_rsae_sha384",
    "rsa_pss_rsae_sha512",
    "rsa_pkcs1_sha256",
    "rsa_pkcs1_sha384",
    "rsa_pkcs1_sha512",
    "ecdsa_sha1",
    "rsa_pkcs1_sha1"
);

const CERT_COMPRESSION_ALGORITHM: &[CertCompressionAlgorithm] = &[
    CertCompressionAlgorithm::Zlib,
    CertCompressionAlgorithm::Brotli,
    CertCompressionAlgorithm::Zstd,
];

const DELEGATED_CREDENTIALS: &str = join!(
    ":",
    "ecdsa_secp256r1_sha256",
    "ecdsa_secp384r1_sha384",
    "ecdsa_secp521r1_sha512",
    "ecdsa_sha1"
);

const RECORD_SIZE_LIMIT: u16 = 0x4001;

const EXTENSION_PERMUTATION_INDICES: &[u8] = &{
    const EXTENSIONS: &[ExtensionType] = &[
        ExtensionType::SERVER_NAME,
        ExtensionType::EXTENDED_MASTER_SECRET,
        ExtensionType::RENEGOTIATE,
        ExtensionType::SUPPORTED_GROUPS,
        ExtensionType::EC_POINT_FORMATS,
        ExtensionType::SESSION_TICKET,
        ExtensionType::APPLICATION_LAYER_PROTOCOL_NEGOTIATION,
        ExtensionType::STATUS_REQUEST,
        ExtensionType::DELEGATED_CREDENTIAL,
        ExtensionType::KEY_SHARE,
        ExtensionType::SUPPORTED_VERSIONS,
        ExtensionType::SIGNATURE_ALGORITHMS,
        ExtensionType::PSK_KEY_EXCHANGE_MODES,
        ExtensionType::RECORD_SIZE_LIMIT,
        ExtensionType::CERT_COMPRESSION,
        ExtensionType::ENCRYPTED_CLIENT_HELLO,
    ];

    let mut indices = [0u8; EXTENSIONS.len()];
    let mut index = usize::MIN;
    while index < EXTENSIONS.len() {
        if let Some(idx) = ExtensionType::index_of(EXTENSIONS[index]) {
            indices[index] = idx as u8;
        }
        index += 1;
    }

    indices
};

const HEADER_ORDER: &[HeaderName] = &[
    header::USER_AGENT,
    header::ACCEPT_LANGUAGE,
    header::ACCEPT_ENCODING,
    header::COOKIE,
    header::HOST,
];

#[tokio::main]
async fn main() -> Result<(), rquest::Error> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("trace"));

    // TLS settings
    let tls = TlsSettings::builder()
        .curves(CURVES)
        .cipher_list(CIPHER_LIST)
        .sigalgs_list(SIGALGS_LIST)
        .delegated_credentials(DELEGATED_CREDENTIALS)
        .cert_compression_algorithm(CERT_COMPRESSION_ALGORITHM)
        .record_size_limit(RECORD_SIZE_LIMIT)
        .alpn_protos(AlpnProtos::All)
        .alps_protos(AlpsProtos::Http2)
        .pre_shared_key(true)
        .enable_ech_grease(true)
        .min_tls_version(TlsVersion::TLS_1_0)
        .max_tls_version(TlsVersion::TLS_1_3)
        .extension_permutation_indices(EXTENSION_PERMUTATION_INDICES)
        .build();

    // HTTP/2 settings
    let http2 = Http2Settings::builder()
        .initial_stream_id(15)
        .header_table_size(65536)
        .initial_stream_window_size(131072)
        .max_frame_size(16384)
        .initial_connection_window_size(12517377 + 65535)
        .headers_priority(StreamDependency::new(StreamId::from(13), 41, false))
        .headers_pseudo_order([Method, Scheme, Authority, Path])
        .settings_order([
            HeaderTableSize,
            EnablePush,
            MaxConcurrentStreams,
            InitialWindowSize,
            MaxFrameSize,
            MaxHeaderListSize,
            UnknownSetting8,
            UnknownSetting9,
        ])
        .priority(vec![
            Priority::new(
                StreamId::from(3),
                StreamDependency::new(StreamId::zero(), 200, false),
            ),
            Priority::new(
                StreamId::from(5),
                StreamDependency::new(StreamId::zero(), 100, false),
            ),
            Priority::new(
                StreamId::from(7),
                StreamDependency::new(StreamId::zero(), 0, false),
            ),
            Priority::new(
                StreamId::from(9),
                StreamDependency::new(StreamId::from(7), 0, false),
            ),
            Priority::new(
                StreamId::from(11),
                StreamDependency::new(StreamId::from(3), 0, false),
            ),
            Priority::new(
                StreamId::from(13),
                StreamDependency::new(StreamId::zero(), 240, false),
            ),
        ])
        .build();

    // Headers
    let headers = {
        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, HeaderValue::from_static("rquest"));
        headers.insert(
            header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US,en;q=0.9"),
        );
        headers.insert(
            header::ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br"),
        );
        headers.insert(header::HOST, HeaderValue::from_static("tls.peet.ws"));
        headers.insert(header::COOKIE, HeaderValue::from_static("foo=bar"));
        headers
    };

    // Create impersonate settings
    let settings = ImpersonateSettings::builder()
        .tls(tls)
        .http2(http2)
        .headers(headers)
        .headers_order(HEADER_ORDER)
        .build();

    // Build a client with impersonate settings
    let client = Client::builder()
        .impersonate(settings)
        .danger_accept_invalid_certs(true)
        .build()?;

    // Use the API you're already familiar with
    let text = client
        .get("https://tls.peet.ws/api/all")
        .send()
        .await?
        .text()
        .await?;

    println!("{}", text);

    Ok(())
}
