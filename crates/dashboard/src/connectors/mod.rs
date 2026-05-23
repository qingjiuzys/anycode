//! Read-only external connector previews (V2 POC).

pub mod github;
pub mod linear;

pub use github::{fetch_github_issues, GithubIssueSummary};
pub use linear::{fetch_linear_issues, LinearIssueSummary};
