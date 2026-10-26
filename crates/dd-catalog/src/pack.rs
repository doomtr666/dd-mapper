/// A pack as its `pack.json` describes it, which maps must declare to use it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub keywords: String,
}
