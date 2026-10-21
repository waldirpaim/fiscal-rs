//! Text formatting helpers (CNPJ/CPF, CEP, access key, document number...).

/// Keeps ASCII letters and digits.
fn only_alnum(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).collect()
}

fn pad_left(s: &str, width: usize, fill: char) -> String {
    let len = s.chars().count();
    if len >= width {
        s.to_owned()
    } else {
        format!("{}{s}", fill.to_string().repeat(width - len))
    }
}

/// `FormatarCNPJouCPF`: masks 11 characters as a CPF and 14 as a CNPJ; anything else is
/// returned untouched.
pub fn cnpj_ou_cpf(value: &str) -> String {
    let s = only_alnum(value);
    match s.chars().count() {
        11 => {
            let s = pad_left(&only_digits(&s), 11, '0');
            format!("{}.{}.{}-{}", &s[0..3], &s[3..6], &s[6..9], &s[9..11])
        }
        14 => {
            format!(
                "{}.{}.{}/{}-{}",
                &s[0..2],
                &s[2..5],
                &s[5..8],
                &s[8..12],
                &s[12..14]
            )
        }
        _ => value.to_owned(),
    }
}

fn only_digits(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

/// `FormatarCEP`: `00000-000`. Shorter numbers are zero padded on the left.
pub fn cep(value: &str) -> String {
    let mut s = only_digits(value);
    if s.len() < 5 {
        s = pad_left(&s, 5, '0');
    }
    if s.len() == 5 {
        s.push_str("000");
    } else {
        s = pad_left(&s, 8, '0');
    }
    format!("{}-{}", &s[0..5], &s[5..8])
}

/// `FormatarCEP` for a numeric CEP (`0` is not formatted by callers).
pub fn cep_number(value: u32) -> String {
    cep(&value.to_string())
}

/// `FormatarChaveAcesso`: the access key in groups of four digits.
pub fn chave_acesso(value: &str) -> String {
    let key = crate::model::remove_literal_chave(value);
    let chars: Vec<char> = key.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len().saturating_sub(1) {
        let end = (i + 4).min(chars.len());
        out.extend(&chars[i..end]);
        out.push(' ');
        i += 4;
    }
    out.trim().to_owned()
}

/// `FormatarNumeroDocumentoFiscal`: `000.000.000`.
pub fn numero_documento_fiscal(value: &str) -> String {
    let s = pad_left(value.trim(), 9, '0');
    format!("{}.{}.{}", &s[0..3], &s[3..6], &s[6..9])
}

/// `FormatFloat('000', v)` for series and similar counters.
pub fn zero_pad(value: u64, width: usize) -> String {
    format!("{value:0width$}")
}

/// Brazilian state abbreviation of an IBGE state code (`CodigoUFparaUF`).
pub fn codigo_uf_para_uf(code: u32) -> &'static str {
    match code {
        11 => "RO",
        12 => "AC",
        13 => "AM",
        14 => "RR",
        15 => "PA",
        16 => "AP",
        17 => "TO",
        21 => "MA",
        22 => "PI",
        23 => "CE",
        24 => "RN",
        25 => "PB",
        26 => "PE",
        27 => "AL",
        28 => "SE",
        29 => "BA",
        31 => "MG",
        32 => "ES",
        33 => "RJ",
        35 => "SP",
        41 => "PR",
        42 => "SC",
        43 => "RS",
        50 => "MS",
        51 => "MT",
        52 => "GO",
        53 => "DF",
        99 => "EX",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_documents() {
        assert_eq!(cnpj_ou_cpf("11222333000181"), "11.222.333/0001-81");
        assert_eq!(cnpj_ou_cpf("12345678909"), "123.456.789-09");
        assert_eq!(cnpj_ou_cpf("11.222.333/0001-81"), "11.222.333/0001-81");
        assert_eq!(cnpj_ou_cpf("123"), "123");
        assert_eq!(cnpj_ou_cpf(""), "");
        assert_eq!(cnpj_ou_cpf("12ABC34500DE81"), "12.ABC.345/00DE-81");
    }

    #[test]
    fn masks_cep() {
        assert_eq!(cep("78895350"), "78895-350");
        assert_eq!(cep_number(1001000), "01001-000");
        assert_eq!(cep("18270"), "18270-000");
        assert_eq!(cep("9876"), "09876-000");
        assert_eq!(cep("9876000"), "09876-000");
    }

    #[test]
    fn groups_the_access_key() {
        assert_eq!(
            chave_acesso("NFe35260711222333000181550010000012341123456780"),
            "3526 0711 2223 3300 0181 5500 1000 0012 3411 2345 6780"
        );
        // The original loop stops before a final lone character.
        assert_eq!(chave_acesso("12345"), "1234");
        assert_eq!(chave_acesso("123456"), "1234 56");
    }

    #[test]
    fn formats_document_numbers() {
        assert_eq!(numero_documento_fiscal("1234"), "000.001.234");
        assert_eq!(numero_documento_fiscal("123456789"), "123.456.789");
        assert_eq!(zero_pad(7, 3), "007");
        assert_eq!(zero_pad(1234, 3), "1234");
    }

    #[test]
    fn maps_state_codes() {
        assert_eq!(codigo_uf_para_uf(35), "SP");
        assert_eq!(codigo_uf_para_uf(51), "MT");
        assert_eq!(codigo_uf_para_uf(0), "");
    }
}
