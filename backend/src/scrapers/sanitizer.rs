use std::str::FromStr;

use rust_decimal::Decimal;

use crate::models::SearchTarget;

/// "R$ 5.499,90" -> 5499.90. Ponto é milhar, vírgula é decimal.
pub fn parse_brl(raw: &str) -> Option<Decimal> {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    Decimal::from_str(&cleaned).ok()
}

/// Minúsculas, sem acentos, só tokens alfanuméricos, com espaço nas pontas
/// para permitir match de token isolado via `contains(" kw ")`.
pub fn normalize(s: &str) -> String {
    let mapped: String = s
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            c if c.is_ascii_alphanumeric() => c,
            _ => ' ',
        })
        .collect();
    format!(" {} ", mapped.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn has_keyword(normalized: &str, kw: &str) -> bool {
    let kw = normalize(kw);
    let kw = kw.trim();
    !kw.is_empty() && normalized.contains(&format!(" {kw} "))
}

pub fn is_valid(title: &str, price: Decimal, target: &SearchTarget) -> bool {
    let t = normalize(title);

    if !target.required_keywords.iter().all(|kw| has_keyword(&t, kw)) {
        return false;
    }
    if target.excluded_keywords.iter().any(|kw| has_keyword(&t, kw)) {
        return false;
    }
    // Alvo base (ex.: M5) não aceita variantes Pro/Max.
    if !target.chip_family.to_lowercase().contains("pro") && (has_keyword(&t, "pro") || has_keyword(&t, "max")) {
        return false;
    }
    price >= target.min_price_brl && target.max_price_brl.map_or(true, |max| price <= max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    const EXCLUDED: &[&str] = &[
        "hub", "dock", "suporte", "support", "stand", "case", "capa", "adesivo", "skin", "cabo", "fonte",
        "carregador", "caixa vazia", "defeito", "pecas", "peças", "m1", "m2", "m3", "m4", "intel", "i5", "i7",
    ];

    fn target(chip: &str, required: &[&str], min: i64) -> SearchTarget {
        SearchTarget {
            id: Uuid::new_v4(),
            name: format!("Mac mini {chip}"),
            chip_family: chip.to_string(),
            search_query: format!("mac mini {}", chip.to_lowercase()),
            required_keywords: required.iter().map(|s| s.to_string()).collect(),
            excluded_keywords: EXCLUDED.iter().map(|s| s.to_string()).collect(),
            min_price_brl: Decimal::from(min),
            max_price_brl: None,
            is_active: true,
            created_at: Utc::now(),
        }
    }

    fn m5() -> SearchTarget {
        target("M5", &["mac", "mini", "m5"], 3800)
    }
    fn m5_pro() -> SearchTarget {
        target("M5 Pro", &["mac", "mini", "m5", "pro"], 6500)
    }
    fn m6() -> SearchTarget {
        target("M6", &["mac", "mini", "m6"], 4200)
    }
    fn brl(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn aprova_m6_valido() {
        assert!(is_valid("Apple Mac mini M6 16GB RAM 256GB SSD", brl("5999"), &m6()));
    }

    #[test]
    fn reprova_acessorio() {
        let t = "Hub USB-C Suporte para Mac mini M5 M6";
        assert!(!is_valid(t, brl("9000"), &m5()));
        assert!(!is_valid(t, brl("9000"), &m6()));
    }

    #[test]
    fn reprova_chip_antigo() {
        assert!(!is_valid("Mac mini M2 8GB", brl("9000"), &m5()));
        assert!(!is_valid("Mac mini M2 8GB", brl("9000"), &m6()));
    }

    #[test]
    fn m5_pro_so_no_alvo_pro() {
        let t = "Mac mini M5 Pro 24GB";
        assert!(!is_valid(t, brl("9000"), &m5()));
        assert!(is_valid(t, brl("9000"), &m5_pro()));
    }

    #[test]
    fn faixa_de_preco() {
        let mut t = m6();
        t.max_price_brl = Some(brl("8000"));
        let title = "Apple Mac mini M6 16GB";
        assert!(!is_valid(title, brl("4199.99"), &t));
        assert!(!is_valid(title, brl("8000.01"), &t));
        assert!(is_valid(title, brl("8000"), &t));
    }

    #[test]
    fn aceita_acentos() {
        assert!(is_valid("Mác Míni M5", brl("4000"), &m5()));
    }

    #[test]
    fn token_isolado() {
        assert!(!is_valid("Mac mini M50", brl("4000"), &m5()));
        assert_eq!(normalize("Apple Mac-Mini (M5)"), " apple mac mini m5 ");
    }

    #[test]
    fn parse_brl_casos() {
        assert_eq!(parse_brl("R$ 5.499,90"), Some(brl("5499.90")));
        assert_eq!(parse_brl("5499,00"), Some(brl("5499.00")));
        assert_eq!(parse_brl("R$ 6.200"), Some(brl("6200")));
        assert_eq!(parse_brl(""), None);
        assert_eq!(parse_brl("R$"), None);
    }
}
