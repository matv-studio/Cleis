//! Text normalisation used for matching (import headers, search, duplicates).

/// Lower-case, strip common Latin accents, collapse whitespace.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = true;
    for c in s.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'œ' => {
                out.push('o');
                'e'
            }
            'æ' => {
                out.push('a');
                'e'
            }
            '’' | '\'' => ' ',
            c => c,
        };
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out.trim_end().to_owned()
}

/// Case- and accent-insensitive substring search on every word of `query`.
pub fn matches(haystack: &str, query: &str) -> bool {
    let h = fold(haystack);
    fold(query)
        .split(' ')
        .filter(|w| !w.is_empty())
        .all(|w| h.contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds() {
        assert_eq!(fold("  Coût   Total "), "cout total");
        assert_eq!(fold("Lefèvre"), "lefevre");
        assert!(matches("Paul Lefèvre — Example Bakery", "lefevre bak"));
        assert!(!matches("Paul", "paula"));
    }
}
