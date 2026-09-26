use super::{BreakKind, LineBreakClass::*, Token};
pub(super) fn boundary(tokens: &[Token], right: usize) -> Option<BreakKind> {
    let a = tokens[right - 1];
    let b = tokens[right];
    let (l, r) = (a.class(), b.class());
    let before = right.checked_sub(2).map(|i| tokens[i]);
    let after = tokens.get(right + 1).copied();
    let nonspace = b.previous_non_space.map(|i| tokens[i]);
    if l == Wj || r == Wj || l == Gl {
        return None;
    } // LB11/12
    if r == Gl && !matches!(l, Sp | Hy | Hh) {
        return None;
    } // LB12a rev.57
    if matches!(r, Cl | Cp | Ex | Sy) {
        return None;
    } // LB13
    if nonspace.is_some_and(|t| t.class() == Op) {
        return None;
    } // LB14
    if let Some(q) = b.previous_non_space {
        let quote = tokens[q];
        if quote.class() == Qu
            && quote.props.initial_punctuation
            && (q == 0
                || matches!(
                    tokens[q - 1].class(),
                    Bk | Cr | Lf | Nl | Op | Qu | Gl | Sp | Zw
                ))
        {
            return None;
        } // LB15a
    }
    if r == Qu
        && b.props.final_punctuation
        && after.is_none_or(|t| {
            matches!(
                t.class(),
                Sp | Gl | Wj | Cl | Qu | Cp | Ex | Is | Sy | Bk | Cr | Lf | Nl | Zw
            )
        })
    {
        return None;
    } // LB15b
    if l == Sp && r == Is && after.is_some_and(|t| t.class() == Nu) {
        return Some(BreakKind::Allowed);
    } // LB15c
    if r == Is {
        return None;
    } // LB15d
    if r == Ns && nonspace.is_some_and(|t| matches!(t.class(), Cl | Cp)) {
        return None;
    } // LB16
    if r == B2 && nonspace.is_some_and(|t| t.class() == B2) {
        return None;
    } // LB17
    if l == Sp {
        return Some(BreakKind::Allowed);
    } // LB18
    if (r == Qu && !b.props.initial_punctuation) || (l == Qu && !a.props.final_punctuation) {
        return None;
    } // LB19
    if (r == Qu && (!a.props.east_asian || after.is_none_or(|t| !t.props.east_asian)))
        || (l == Qu && (!b.props.east_asian || before.is_none_or(|t| !t.props.east_asian)))
    {
        return None;
    } // LB19a
    if l == Cb || r == Cb {
        return Some(BreakKind::Allowed);
    } // LB20
    if matches!(l, Hy | Hh)
        && matches!(r, Al | Hl)
        && before.is_none_or(|t| matches!(t.class(), Bk | Cr | Lf | Nl | Sp | Zw | Cb | Gl))
    {
        return None;
    } // LB20a
    if matches!(r, Ba | Hh | Hy | Ns) || l == Bb {
        return None;
    } // LB21
    if matches!(l, Hy | Hh) && r != Hl && before.is_some_and(|t| t.class() == Hl) {
        return None;
    } // LB21a
    if l == Sy && r == Hl {
        return None;
    } // LB21b
    if r == In {
        return None;
    } // LB22
    if (matches!(l, Al | Hl) && r == Nu) || (l == Nu && matches!(r, Al | Hl)) {
        return None;
    } // LB23
    if (l == Pr && matches!(r, Id | Eb | Em)) || (matches!(l, Id | Eb | Em) && r == Po) {
        return None;
    } // LB23a
    if (matches!(l, Pr | Po) && matches!(r, Al | Hl))
        || (matches!(l, Al | Hl) && matches!(r, Pr | Po))
    {
        return None;
    } // LB24
    if matches!(r, Po | Pr)
        && (a.numeric_suffix || (matches!(l, Cl | Cp) && before.is_some_and(|t| t.numeric_suffix)))
    {
        return None;
    } // LB25 suffix
    if matches!(l, Po | Pr)
        && (r == Nu
            || (r == Op
                && after.is_some_and(|t| {
                    t.class() == Nu
                        || (t.class() == Is
                            && tokens.get(right + 2).is_some_and(|n| n.class() == Nu))
                })))
    {
        return None;
    }
    if r == Nu && (matches!(l, Hy | Is) || a.numeric_suffix) {
        return None;
    }
    if (l == Jl && matches!(r, Jl | Jv | H2 | H3))
        || (matches!(l, Jv | H2) && matches!(r, Jv | Jt))
        || (matches!(l, Jt | H3) && r == Jt)
    {
        return None;
    } // LB26
    if (matches!(l, Jl | Jv | Jt | H2 | H3) && r == Po)
        || (l == Pr && matches!(r, Jl | Jv | Jt | H2 | H3))
    {
        return None;
    } // LB27
    if matches!(l, Al | Hl) && matches!(r, Al | Hl) {
        return None;
    } // LB28
    if (l == Ap && b.brahmic())
        || (a.brahmic() && matches!(r, Vf | Vi))
        || (l == Vi && (r == Ak || b.dotted_circle) && before.is_some_and(Token::brahmic))
        || (a.brahmic() && b.brahmic() && after.is_some_and(|t| t.class() == Vf))
    {
        return None;
    } // LB28a
    if l == Is && matches!(r, Al | Hl) {
        return None;
    } // LB29
    if (matches!(l, Al | Hl | Nu) && r == Op && !b.props.east_asian)
        || (l == Cp && !a.props.east_asian && matches!(r, Al | Hl | Nu))
    {
        return None;
    } // LB30
    if l == Ri && r == Ri && a.ri_odd {
        return None;
    } // LB30a
    if r == Em && (l == Eb || a.potential_emoji) {
        return None;
    } // LB30b
    Some(BreakKind::Allowed) // LB31
}
