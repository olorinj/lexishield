//! Lista de palabras reservadas, stopwords y términos protegidos para evitar falsos positivos y colisiones.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Conjunto estático e inmutable de palabras reservadas y stopwords en español e inglés.
pub static PROTECTED_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let words = [
        // Stopwords comunes en español
        "de",
        "la",
        "que",
        "el",
        "en",
        "y",
        "a",
        "los",
        "se",
        "del",
        "las",
        "por",
        "un",
        "para",
        "con",
        "no",
        "una",
        "su",
        "al",
        "lo",
        "como",
        "mas",
        "pero",
        "sus",
        "le",
        "ya",
        "o",
        "este",
        "si",
        "porque",
        "esta",
        "entre",
        "cuando",
        "muy",
        "sin",
        "sobre",
        "tambien",
        "me",
        "hasta",
        "hay",
        "donde",
        "quien",
        "desde",
        "todo",
        "nos",
        "durante",
        "todos",
        "uno",
        "les",
        "ni",
        "contra",
        "otros",
        "ese",
        "eso",
        "ante",
        "ellos",
        "e",
        "esto",
        "mi",
        "antes",
        "algunos",
        "que",
        "unos",
        "yo",
        "otro",
        "otras",
        "otra",
        "el",
        "tanto",
        "esa",
        "estos",
        "mucho",
        "quienes",
        "nada",
        "muchos",
        "cual",
        "poco",
        "ella",
        "estar",
        "estas",
        "algunas",
        "algo",
        "nosotros",
        "mi",
        "mis",
        "tu",
        "te",
        "ti",
        "tus",
        "ellas",
        "nosotras",
        "vosostros",
        "vosostras",
        "os",
        "mio",
        "mia",
        "mios",
        "mias",
        "tuyo",
        "tuya",
        // Stopwords comunes en inglés
        "the",
        "be",
        "to",
        "of",
        "and",
        "a",
        "in",
        "that",
        "have",
        "i",
        "it",
        "for",
        "not",
        "on",
        "with",
        "he",
        "as",
        "you",
        "do",
        "at",
        "this",
        "but",
        "his",
        "by",
        "from",
        "they",
        "we",
        "say",
        "her",
        "she",
        "or",
        "an",
        "will",
        "my",
        "one",
        "all",
        "would",
        "there",
        "their",
        "what",
        "so",
        "up",
        "out",
        "if",
        "about",
        "who",
        "get",
        "which",
        "go",
        "me",
        "when",
        "make",
        "can",
        "like",
        "time",
        "no",
        "just",
        "him",
        "know",
        "take",
        "people",
        "into",
        "year",
        "your",
        "good",
        "some",
        "could",
        "them",
        "see",
        "other",
        "than",
        "then",
        "now",
        "look",
        "only",
        "come",
        "its",
        "over",
        "think",
        "also",
        "back",
        "after",
        "use",
        "two",
        "how",
        "our",
        "work",
        "first",
        "well",
        "way",
        "even",
        "new",
        "want",
        "because",
        "any",
        "these",
        "give",
        "day",
        "most",
        "us",
        // Palabras reservadas de programación y sintaxis (SQL, Python, Rust, Bash, JS)
        "fn",
        "let",
        "mut",
        "pub",
        "struct",
        "enum",
        "impl",
        "trait",
        "match",
        "loop",
        "while",
        "for",
        "if",
        "else",
        "return",
        "break",
        "continue",
        "true",
        "false",
        "null",
        "nil",
        "def",
        "class",
        "import",
        "from",
        "as",
        "try",
        "except",
        "finally",
        "raise",
        "with",
        "async",
        "await",
        "lambda",
        "yield",
        "pass",
        "select",
        "insert",
        "update",
        "delete",
        "drop",
        "table",
        "create",
        "alter",
        "where",
        "group",
        "order",
        "having",
        "join",
        "inner",
        "outer",
        "left",
        "right",
        "union",
        "distinct",
        "count",
        "sum",
        "avg",
        "max",
        "min",
        "primary",
        "foreign",
        "key",
        "references",
        "default",
        "check",
        "index",
        // Métodos y cabeceras HTTP comunes
        "http",
        "https",
        "get",
        "post",
        "put",
        "delete",
        "patch",
        "head",
        "options",
        "trace",
        "connect",
        "host",
        "user-agent",
        "accept",
        "content-type",
        "content-length",
        "authorization",
        "cookie",
        "set-cookie",
        "referer",
        "origin",
        "status",
        "headers",
        "body",
    ];

    let mut set = HashSet::with_capacity(words.len() * 2);
    for word in words {
        set.insert(word);
    }
    set
});

/// Determina si una palabra está protegida y no debe ser reemplazada genéricamente.
pub fn is_protected_word(word: &str) -> bool {
    let lower = word.to_lowercase();
    PROTECTED_WORDS.contains(lower.as_str())
}

/// SIDs universales y conocidos de Windows que nunca deben ofuscarse.
pub static PROTECTED_SIDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let sids = [
        "S-1-0-0",  // Null Authority / Nobody
        "S-1-1-0",  // World / Everyone
        "S-1-2-0",  // Local
        "S-1-3-0",  // Creator Owner
        "S-1-3-1",  // Creator Group
        "S-1-5-18", // Local System
        "S-1-5-19", // NT Authority / Local Service
        "S-1-5-20", // NT Authority / Network Service
    ];
    let mut set = HashSet::with_capacity(sids.len() * 2);
    for sid in sids {
        set.insert(sid);
    }
    set
});

/// Determina si un SID es un identificador estándar de Windows protegido.
pub fn is_protected_sid(sid: &str) -> bool {
    PROTECTED_SIDS.contains(sid)
}

/// Dominios de empresas de software conocidas, servicios en la nube e infraestructura pública
/// que pueden permanecer sin ofuscar porque no contienen información confidencial del cliente.
pub static KNOWN_VENDOR_DOMAINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let domains = [
        // Microsoft / Windows / Azure / Office / O365
        "microsoft.com",
        "microsoftonline.com",
        "windows.com",
        "windows.net",
        "office.com",
        "office365.com",
        "outlook.com",
        "live.com",
        "azure.com",
        "msft.net",
        "trafficmanager.net",
        "msn.com",
        "visualstudio.com",
        "azureedge.net",
        "sharepoint.com",
        "onedrive.com",
        "skype.com",
        "bing.com",
        "microsoftstream.com",
        "lync.com",
        "yammer.com",
        // Google / Alphabet
        "google.com",
        "googleapis.com",
        "gstatic.com",
        "googleusercontent.com",
        "gmail.com",
        "gvt1.com",
        "android.com",
        "chromium.org",
        // Amazon / AWS
        "amazon.com",
        "amazonaws.com",
        "cloudfront.net",
        // Cloudflare / CDNs públicas
        "cloudflare.com",
        "cloudflare-dns.com",
        "fastly.net",
        "akamai.net",
        "akamaiedge.net",
        // GitHub / GitLab
        "github.com",
        "githubusercontent.com",
        "gitlab.com",
        // Seguridad, SIEM y Sistemas
        "wazuh.com",
        "elastic.co",
        "splunk.com",
        "crowdstrike.com",
        "sentinelone.com",
        "virustotal.com",
        "debian.org",
        "ubuntu.com",
        "redhat.com",
        "centos.org",
        "kernel.org",
        "gnu.org",
        "apache.org",
        // Apple
        "apple.com",
        "icloud.com",
        "apple-dns.net",
        // CA / PKI / NTP públicas
        "digicert.com",
        "letsencrypt.org",
        "verisign.com",
        "sectigo.com",
        "entrust.net",
        "pool.ntp.org",
    ];
    let mut set = HashSet::with_capacity(domains.len() * 2);
    for d in domains {
        set.insert(d);
    }
    set
});

/// Determina si un dominio pertenece a un proveedor de software / cloud conocido.
pub fn is_known_vendor_domain(domain: &str) -> bool {
    let lower = domain.trim().trim_end_matches('.').to_lowercase();
    if KNOWN_VENDOR_DOMAINS.contains(lower.as_str()) {
        return true;
    }
    // Comprobar si es un subdominio de alguno de los vendors
    for vendor in KNOWN_VENDOR_DOMAINS.iter() {
        if lower.ends_with(&format!(".{vendor}")) {
            return true;
        }
    }
    // Dominios de ejemplo RFC 2606 o ya anonimizados previamente
    if lower == "example.com" || lower.ends_with(".example.com") {
        return true;
    }
    false
}

/// Determina si un dominio o FQDN es de Active Directory o red corporativa privada que DEBE ofuscarse.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
pub fn is_ad_or_private_domain(domain: &str) -> bool {
    let lower = domain.trim().trim_end_matches('.').to_lowercase();
    lower.ends_with(".local")
        || lower.ends_with(".internal")
        || lower.ends_with(".corp")
        || lower.ends_with(".lan")
        || lower.ends_with(".home")
        || lower.ends_with(".priv")
        || lower.ends_with(".ad")
        || lower.ends_with(".intranet")
        || !is_known_vendor_domain(&lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_protected_word() {
        assert!(is_protected_word("SELECT"));
        assert!(is_protected_word("select"));
        assert!(is_protected_word("authorization"));
        assert!(is_protected_word("para"));
        assert!(is_protected_word("HTTP"));
        assert!(is_protected_word("https"));
        assert!(!is_protected_word("juan_perez_99"));
        assert!(!is_protected_word("supersecretpassword"));
    }

    #[test]
    fn test_known_vendor_and_ad_domains() {
        assert!(is_known_vendor_domain("microsoft.com"));
        assert!(is_known_vendor_domain("login.microsoftonline.com"));
        assert!(is_known_vendor_domain("outlook.office365.com"));
        assert!(is_known_vendor_domain("wazuh.com"));
        assert!(is_known_vendor_domain("google.com"));
        assert!(is_known_vendor_domain("s3.eu-west-1.amazonaws.com"));

        // Dominios corporativos o de Active Directory NO son proveedores de software
        assert!(!is_known_vendor_domain("acme.local"));
        assert!(!is_known_vendor_domain("corp.miempresa.com"));
        assert!(!is_known_vendor_domain("banco.es"));
        assert!(!is_known_vendor_domain("server01.ad.hospital.org"));

        // Deben ser clasificados como AD / dominios privados para ofuscación obligatoria
        assert!(is_ad_or_private_domain("acme.local"));
        assert!(is_ad_or_private_domain("corp.miempresa.com"));
        assert!(is_ad_or_private_domain("ad.empresa.lan"));
        assert!(is_ad_or_private_domain("banco.es"));
        assert!(!is_ad_or_private_domain("microsoft.com"));
        assert!(!is_ad_or_private_domain("windows.net"));
    }
}
