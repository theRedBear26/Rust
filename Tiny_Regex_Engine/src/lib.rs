
fn is_alpha_class(expr: &str) -> bool {
    expr == r"\a"
}

fn is_digit_class(expr: &str) -> bool {
    expr == r"\d"
}

fn is_start(expr: &str) -> bool {
    expr.starts_with('^')
}

fn is_stop(expr: &str) -> bool {
    expr == "$"
}

fn is_literal(c: char) -> bool {
    !matches!(
        c,
        '\\' | '*' | '+' | '?' | '[' | ']' | '(' | ')' | '^' | '$' | '|'
    ) && c != '.'
}

fn is_set(expr: &str) -> bool {
    expr.starts_with('[') && expr.ends_with(']')
}

fn is_group(expr: &str) -> bool {
    expr.starts_with('(') && expr.ends_with(')')
}

fn is_operator(c: char) -> bool {
    matches!(c, '*' | '+' | '?')
}

fn split_set(expr: &str) -> &str {
    &expr[1..expr.len() - 1]
}

fn split_group(expr: &str) -> Vec<&str> {
    expr[1..expr.len() - 1].split('|').collect()
}

fn does_unit_match(unit: &str, text: &str) -> Option<usize> {
    if is_group(unit) {
        for alt in split_group(unit) {
            if let Some(n) = match_expr(alt, text, 0) {
                return Some(n);
            }
        }
        return None;
    }

    let t = text.chars().next()?;
    let h = unit.chars().next()?;

    let matched = if is_set(unit) {
        split_set(unit).contains(t)
    } else if is_alpha_class(unit) {
        t.is_ascii_alphabetic()
    } else if is_digit_class(unit) {
        t.is_ascii_digit()
    } else if h == '.' {
        true
    } else if is_literal(h) {
        h == t
    } else {
        false
    };

    matched.then_some(t.len_utf8())
}

fn split_expr(expr: &str) -> (Option<&str>, Option<char>, &str) {
    let mut head = None;
    let mut operator = None;
    let mut pos = 0;

    if expr.starts_with('[') {
        if let Some(end) = expr.find(']') {
            pos = end + 1;
            head = Some(&expr[..pos]);
        }
    } else if expr.starts_with('(') {
        if let Some(end) = expr.find(')') {
            pos = end + 1;
            head = Some(&expr[..pos]);
        }
    } else if expr.starts_with(r"\a") || expr.starts_with(r"\d") {
        pos = 2;
        head = Some(&expr[..pos]);
    } else if let Some(c) = expr.chars().next() {
        if is_literal(c) || c == '.' {
            pos = c.len_utf8();
            head = Some(&expr[..pos]);
        } else if c == '^' {
            pos = 1;
            head = Some(&expr[..pos]);
        }
    }

    if let Some(op) = expr[pos..]
        .chars()
        .next()
        .filter(|&c| is_operator(c))
    {
        operator = Some(op);
        pos += op.len_utf8();
    }

    (head, operator, &expr[pos..])
}

fn match_expr(expr: &str, text: &str, length: usize) -> Option<usize> {
    if expr.is_empty() {
        return Some(length);
    }

    if is_stop(expr) {
        return if text.is_empty() {
            Some(length)
        } else {
            None
        };
    }

    let (head, operator, rest) = split_expr(expr);

    let unit = head?;

    let (min, max) = match operator {
        Some('?') => (0, 1),
        Some('*') => (0, usize::MAX),
        Some('+') => (1, usize::MAX),
        _ => (1, 1),
    };

    match_repeat(unit, rest, text, length, min, max)
}

fn match_repeat(
    unit: &str,
    rest: &str,
    text: &str,
    length: usize,
    min: usize,
    max: usize,
) -> Option<usize> {
    let mut ends = vec![0];
    let mut pos = 0;

    while ends.len() - 1 < max {
        let Some(n) = does_unit_match(unit, &text[pos..]) else {
            break;
        };

        if n == 0 {
            break;
        }

        pos += n;
        ends.push(pos);
    }

    for (count, &end) in ends.iter().enumerate().rev() {
        if count < min {
            break;
        }

        if let Some(len) = match_expr(
            rest,
            &text[end..],
            length + end,
        ) {
            return Some(len);
        }
    }

    None
}

pub fn find(
    expr: impl AsRef<str>,
    text: impl AsRef<str>,
) -> Option<(usize, usize)> {
    let expr = expr.as_ref();
    let text = text.as_ref();

    if is_start(expr) {
        if let Some(len) = match_expr(&expr[1..], text, 0) {
            return Some((0, len));
        }
    } else {
        for pos in 0..=text.len() {
            if !text.is_char_boundary(pos) {
                continue;
            }

            if let Some(len) = match_expr(expr, &text[pos..], 0) {
                return Some((pos, len));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn works() {
        assert_eq!(find("abc", "xxabc"), Some((2, 3)));
        assert_eq!(find("a+b", "xaab"), Some((1, 3)));

        assert_eq!(find("colou?r", "color"), Some((0, 5)));
        assert_eq!(find("colou?r", "colour"), Some((0, 6)));

        assert_eq!(find("[abc]+", "zzbca"), Some((2, 3)));
        assert_eq!(find("a*b", "aaab"), Some((0, 4)));
        assert_eq!(find("x", "abc"), None);

        assert_eq!(find("a.b", "ayb"), Some((0, 3)));

        assert_eq!(find("(ab|cb)*", "xcby"), Some((0, 0)));
        assert_eq!(find("(ab|cb)+", "xcby"), Some((1, 2)));
        assert_eq!(find("(ab|cb)+", "abcbab"), Some((0, 6)));
        assert_eq!(find("(a|b)c", "xbc"), Some((1, 2)));

        assert_eq!(find("^x", "xbc"), Some((0, 1)));
        assert_eq!(find("^x*", "bc"), Some((0, 0)));

        assert_eq!(find("ab$", "aaab"), Some((2, 2)));
        assert_eq!(find("a*b$", "aaab"), Some((0, 4)));

        assert_eq!(find(r"\d", "abc123"), Some((3, 1)));
        assert_eq!(find(r"\d\d", "a12b"), Some((1, 2)));
        assert_eq!(find(r"\a\d", "x9"), Some((0, 2)));
        assert_eq!(find(r"\a*\d", "abc7"), Some((0, 4)));
        assert_eq!(find(r"\d*", "abc"), Some((0, 0)));
        assert_eq!(find(r"\a", "ab"), Some((0, 1)));

        assert_eq!(find(r"^\d", "a1"), None);
        assert_eq!(find(r"\d$", "ab3"), Some((2, 1)));

        assert_eq!(find(r"\d+", "hello123"), Some((5, 3)));

        assert_eq!(find("x", "abc"), None);
        assert_eq!(find(r"\d", ""), None);
        assert_eq!(find("", "abc"), Some((0, 0)));
        assert_eq!(find("^$", ""), Some((0, 0)));
        assert_eq!(find("$", "abc"), Some((3, 0)));

        assert_eq!(find("a:b", "xa:b"), Some((1, 3)));
        assert_eq!(find("b", "éb"), Some((2, 1)));
        assert_eq!(find("a!b", "xa!b"), Some((1, 3)));
    }
}
