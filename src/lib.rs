pub mod raftnode;
pub mod storage;
mod message;
pub mod network;
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
