use encoding_rs::Encoding;

static ENCODINGS: &[&Encoding] = &[
    encoding_rs::BIG5,
    encoding_rs::EUC_JP,
    encoding_rs::EUC_KR,
    encoding_rs::GB18030,
    encoding_rs::GBK,
    encoding_rs::IBM866,
    encoding_rs::ISO_2022_JP,
    encoding_rs::ISO_8859_2,
    encoding_rs::ISO_8859_3,
    encoding_rs::ISO_8859_4,
    encoding_rs::ISO_8859_5,
    encoding_rs::ISO_8859_6,
    encoding_rs::ISO_8859_7,
    encoding_rs::ISO_8859_8,
    encoding_rs::ISO_8859_8_I,
    encoding_rs::ISO_8859_10,
    encoding_rs::ISO_8859_13,
    encoding_rs::ISO_8859_14,
    encoding_rs::ISO_8859_15,
    encoding_rs::ISO_8859_16,
    encoding_rs::KOI8_R,
    encoding_rs::KOI8_U,
    encoding_rs::MACINTOSH,
    encoding_rs::REPLACEMENT,
    encoding_rs::SHIFT_JIS,
    encoding_rs::UTF_8,
    encoding_rs::UTF_16BE,
    encoding_rs::UTF_16LE,
    encoding_rs::WINDOWS_874,
    encoding_rs::WINDOWS_1250,
    encoding_rs::WINDOWS_1251,
    encoding_rs::WINDOWS_1252,
    encoding_rs::WINDOWS_1253,
    encoding_rs::WINDOWS_1254,
    encoding_rs::WINDOWS_1255,
    encoding_rs::WINDOWS_1256,
    encoding_rs::WINDOWS_1257,
    encoding_rs::WINDOWS_1258,
    encoding_rs::X_MAC_CYRILLIC,
    encoding_rs::X_USER_DEFINED,
];

pub fn get_encoding(name: &str) -> Option<&'static Encoding> {
    ENCODINGS
        .iter()
        .find(|&encoding| encoding.name() == name)
        .copied()
}

pub fn get_encoding_names() -> Vec<&'static str> {
    ENCODINGS.iter().map(|encoding| encoding.name()).collect()
}
