//! Emergency kit (screen 23): a one-page PDF to print, written directly (PDF 1.4, text in the
//! standard Helvetica font, no images). It holds where the vault is and a space to write the
//! master password by hand; never the password itself.

use std::fmt::Write as _;
use std::path::Path;

/// Escapes a string for a PDF literal; characters outside Latin-1 become `?` (WinAnsi).
fn lit(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('(');
    for ch in s.chars() {
        match ch {
            '(' | ')' | '\\' => {
                o.push('\\');
                o.push(ch);
            }
            '·' => o.push_str("\\267"),
            '—' => o.push_str("\\227"),
            '’' => o.push('\''),
            c if (c as u32) < 0x80 => o.push(c),
            c if (c as u32) <= 0xff => {
                let _ = write!(o, "\\{:03o}", c as u32);
            }
            _ => o.push('?'),
        }
    }
    o.push(')');
    o
}

/// Splits a long path into lines of at most `n` characters.
fn wrap(s: &str, n: usize) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    chars.chunks(n).map(|c| c.iter().collect()).collect()
}

pub fn emergency_kit(vault: &Path, place: Option<&str>, now: i64) -> Vec<u8> {
    let mut c = String::new();
    let mut y = 780.0_f32;
    let text = |c: &mut String, size: f32, bold: bool, s: &str, y: &mut f32| {
        let f = if bold { "F2" } else { "F1" };
        let _ = writeln!(c, "BT /{f} {size} Tf 56 {y:.1} Td {} Tj ET", lit(s));
        *y -= size * 1.6;
    };
    text(&mut c, 22., true, "Cleis — Emergency kit", &mut y);
    text(
        &mut c,
        10.,
        false,
        &format!("Made on {}", crate::format::date(now)),
        &mut y,
    );
    y -= 14.;
    text(&mut c, 12., true, "Vault on this computer", &mut y);
    for l in wrap(&vault.display().to_string(), 80) {
        text(&mut c, 10., false, &l, &mut y);
    }
    y -= 8.;
    text(&mut c, 12., true, "Sync place", &mut y);
    for l in wrap(place.unwrap_or("None"), 80) {
        text(&mut c, 10., false, &l, &mut y);
    }
    y -= 8.;
    text(&mut c, 12., true, "Master password", &mut y);
    text(
        &mut c,
        10.,
        false,
        "Write it by hand. Without it, nobody can open the vault, Cleis included.",
        &mut y,
    );
    y -= 30.;
    let _ = writeln!(c, "0.5 w 56 {y:.1} m 540 {y:.1} l S");
    y -= 36.;
    text(&mut c, 12., true, "On a new computer", &mut y);
    for l in [
        "1. Install Cleis.",
        "2. Choose Restore my vault, then the sync place above.",
        "3. Type the master password.",
    ] {
        text(&mut c, 10., false, l, &mut y);
    }
    y -= 12.;
    text(
        &mut c,
        9.,
        false,
        "Keep this page somewhere safe, away from the computer.",
        &mut y,
    );

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{c}endstream", c.len()),
    ];
    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = vec![];
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    let mut x = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for o in offsets {
        let _ = writeln!(x, "{o:010} 00000 n ");
    }
    let _ = write!(
        x,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.extend_from_slice(x.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn kit_is_a_pdf() {
        let pdf = super::emergency_kit(
            std::path::Path::new("C:\\Users\\ines\\AppData\\Cleis\\vault.cleis"),
            Some("OneDrive · C:\\Users\\ines\\OneDrive\\Cleis\\vault.cleis"),
            1_795_000_000,
        );
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.starts_with("%PDF-1.4"));
        assert!(s.contains("(OneDrive \\267 C:\\\\Users"));
        assert!(s.trim_end().ends_with("%%EOF"));
        // The xref offset points at the table.
        let start: usize = s
            .rsplit("startxref\n")
            .next()
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(pdf[start..].starts_with(b"xref"));
    }
}
