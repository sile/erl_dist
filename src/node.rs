// use std::io::Error;
// use std::net::SocketAddr;
// use futures::{self, Future, BoxFuture};
// use fibers::Spawn;
// use fibers::net::TcpListener;

// use epmd::NodeType;

// #[derive(Debug)]
// pub struct Node {
//     name: String,
//     node_type: NodeType,
// }
// impl Node {}

// // #[derive(Debug, Clone)]
// // pub struct Builder {
// //     listen_addr: Option<SocketAddr>,
// //     epmd_addr: Option<SocketAddr>,
// //     highest_version: u16,
// //     lowest_version: u16,
// //     node_type: NodeType,
// // }
// // impl Builder {
// //     pub fn new() -> Self {
// //         Builder {
// //             listen_addr: None,
// //             epmd_addr: None,
// //             highest_version: 5,
// //             lowest_version: 5,
// //             node_type: NodeType::Normal,
// //         }
// //     }
// //     pub fn listen(&mut self, listen_addr: SocketAddr) -> &mut Self {
// //         self.listen_addr = Some(listen_addr);
// //         self
// //     }
// //     pub fn register(&mut self, epmd_addr: SocketAddr) -> &mut Self {
// //         self.epmd_addr = Some(epmd_addr);
// //         self
// //     }
// //     pub fn version(&mut self, version: u16) -> &mut Self {
// //         self.highest_version(version).lowest_version(version)
// //     }
// //     pub fn highest_version(&mut self, version: u16) -> &mut Self {
// //         self.highest_version = version;
// //         self
// //     }
// //     pub fn lowest_version(&mut self, version: u16) -> &mut Self {
// //         self.lowest_version = version;
// //         self
// //     }
// //     pub fn hidden(&mut self) -> &mut Self {
// //         self.node_type = NodeType::Hidden;
// //         self
// //     }
// //     pub fn finish<T: Spawn + Clone>(&self, spawner: T) -> BoxFuture<Node, Error> {
// //         let Builder { listen_addr, mut epmd_addr, highest_version, lowest_version, node_type } =
// //             self.clone();
// //         if listen_addr.is_none() {
// //             epmd_addr = None;
// //         }
// //         futures::finished(())
// //             .and_then(move |()| {
// //                 listen_addr.map(TcpListener::bind)
// //             })
// //             .boxed()
// //     }
// // }

// // #[derive(Debug)]
// // pub struct Node {
// // }
