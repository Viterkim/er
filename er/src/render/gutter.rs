use super::Gutter;
use core::fmt;

impl Gutter {
    pub(super) fn branch(
        &mut self,
        writer: &mut (impl fmt::Write + ?Sized),
        depth: usize,
        is_last: bool,
    ) -> Result<&str, fmt::Error> {
        // The extra two spaces indent a saved report's body.
        let (branch, padded) = match (depth, is_last) {
            (0, _) => return Ok(""),
            (1, true) => ("\n`- ", "     "),
            (1, false) => ("\n|- ", "|    "),
            (2, true) if self.shallow.starts_with('|') => ("\n|  `- ", "|       "),
            (2, false) if self.shallow.starts_with('|') => ("\n|  |- ", "|  |    "),
            (2, true) => ("\n   `- ", "        "),
            (2, false) => ("\n   |- ", "   |    "),
            _ => return self.deep_branch(writer, depth, is_last),
        };
        writer.write_str(branch)?;
        self.shallow = padded;

        Ok(padded)
    }

    fn deep_branch(
        &mut self,
        writer: &mut (impl fmt::Write + ?Sized),
        depth: usize,
        is_last: bool,
    ) -> Result<&str, fmt::Error> {
        let parent_width = (depth - 1) * 3;
        if depth == 3 {
            self.deep.clear();
            self.deep.reserve(parent_width + 5);
            self.deep.push_str(&self.shallow[..parent_width]);
        } else {
            self.deep.truncate(parent_width);
        }
        let (branch, padded) = if is_last {
            ("`- ", "     ")
        } else {
            ("|- ", "|    ")
        };

        writer.write_char('\n')?;
        writer.write_str(&self.deep)?;
        writer.write_str(branch)?;
        self.deep.push_str(padded);

        Ok(&self.deep)
    }
}
