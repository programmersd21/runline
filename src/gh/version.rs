use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GhVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl GhVersion {
    pub fn parse(output: &str) -> Option<Self> {
        // e.g. "gh version 2.40.1 (2023-12-13)"
        for line in output.lines() {
            if line.contains("gh version") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let ver_str = parts[2];
                    let nums: Vec<&str> = ver_str.split('.').collect();
                    if nums.len() >= 2 {
                        let major = nums[0].parse().ok()?;
                        let minor = nums[1].parse().ok()?;
                        let patch = if nums.len() >= 3 {
                            nums[2].parse().ok().unwrap_or(0)
                        } else {
                            0
                        };
                        return Some(GhVersion {
                            major,
                            minor,
                            patch,
                        });
                    }
                }
            }
        }
        None
    }

    pub fn is_at_least(&self, min_major: u32, min_minor: u32) -> bool {
        if self.major > min_major {
            true
        } else if self.major == min_major {
            self.minor >= min_minor
        } else {
            false
        }
    }
}

impl fmt::Display for GhVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gh_version() {
        let v = GhVersion::parse("gh version 2.40.1 (2023-12-13)").unwrap();
        assert_eq!(
            v,
            GhVersion {
                major: 2,
                minor: 40,
                patch: 1
            }
        );
        assert!(v.is_at_least(2, 40));
        assert!(!v.is_at_least(2, 45));
    }
}
