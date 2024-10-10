use crate::matcher::Matcher;
use crate::Selection;

impl<'a> Selection<'a> {
    /// Checks the current matched set of elements against a selector and
    /// returns true if at least one of these elements matches.
    pub fn is(&self, sel: &str) -> bool {
        if self.length() == 0 {
            return false;
        }
        return Matcher::new(sel)
            .map_or(false,|matcher| self.is_matcher(&matcher))
    }

    /// Checks the current matched set of elements against a matcher and
    /// returns true if at least one of these elements matches.
    pub fn is_matcher(&self, matcher: &Matcher) -> bool {
        if self.length() > 0 {
            return self.nodes().iter().any(|node| matcher.match_element(node));
        }
        false
    }

    /// Checks the current matches set of elements against a selection and
    /// returns true if at least one of these elements matches.
    pub fn is_selection(&self, sel: &Selection) -> bool {
        if self.length() == 0 || sel.length() == 0 {
            return false;
        }
        let m: Vec<usize> = sel.nodes().iter().map(|node| node.id.value).collect();
        self.nodes().iter().any(|node| m.contains(&node.id.value))
    }
}
