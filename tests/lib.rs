extern crate cobalt;

use std::net;
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use cobalt::{Client, Connection, ConnectionID, Config, Handler, Server};

#[test]
fn test_client_connection_failure() {

    let config = Config::default();
    let mut handler = MockClientHandler::new();
    let mut client = Client::new(config);
    client.connect(&mut handler, "127.0.0.1:12345").unwrap();

    assert_eq!(handler.connect_calls, 1);
    assert!(handler.tick_connection_calls > 0);
    assert_eq!(handler.close_calls, 1);

    assert_eq!(handler.connection_calls, 0);
    assert_eq!(handler.connection_failed_calls, 1);
    assert_eq!(handler.connection_congestion_state_calls, 0);
    assert_eq!(handler.connection_packet_lost_calls, 0);
    assert_eq!(handler.connection_lost_calls, 0);

}

#[test]
fn test_server_bind_and_shutdown() {

    let config = Config::default();
    let mut handler = MockServerHandler::new(1, false);
    let mut server = Server::new(config);
    server.bind(&mut handler, "127.0.0.1:0").unwrap();

    assert_eq!(handler.bind_calls, 1);
    assert!(handler.tick_connections_calls > 0);
    assert_eq!(handler.shutdown_calls, 1);

    assert_eq!(handler.connection_calls, 0);
    assert_eq!(handler.connection_failed_calls, 0);
    assert_eq!(handler.connection_congestion_state_calls, 0);
    assert_eq!(handler.connection_packet_lost_calls, 0);
    assert_eq!(handler.connection_lost_calls, 0);

}

#[test]
fn test_server_client_connection_lost() {

    // Get a free local socket and then drop it for quick re-use
    // this is not 100% safe but we cannot easily get the locally bound server
    // address after bind() has been called
    let address: Option<net::SocketAddr> = {
        Some(net::UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap())
    };

    // Setup Test Server
    let server_address = address.clone();
    let server_thread = thread::spawn(move|| {

        let config = Config::default();
        let mut server_handler = MockServerHandler::new(15, false);
        let mut server = Server::new(config);
        server.bind(&mut server_handler, server_address.unwrap()).unwrap();

        assert_eq!(server_handler.bind_calls, 1);
        assert!(server_handler.tick_connections_calls > 0);
        assert_eq!(server_handler.shutdown_calls, 1);

        assert_eq!(server_handler.connection_calls, 1);
        assert_eq!(server_handler.connection_failed_calls, 0);
        assert_eq!(server_handler.connection_congestion_state_calls, 0);
        assert_eq!(server_handler.connection_packet_lost_calls, 0);
        assert_eq!(server_handler.connection_lost_calls, 0);
        assert_eq!(server_handler.connection_closed_calls, 0);

    });

    let config = Config::default();
    let mut client_handler = MockClientHandler::new();
    let mut client = Client::new(config);
    client.connect(&mut client_handler, address.unwrap()).unwrap();

    server_thread.join().unwrap();
    assert_eq!(client_handler.connect_calls, 1);
    assert!(client_handler.tick_connection_calls > 0);
    assert_eq!(client_handler.close_calls, 1);

    assert_eq!(client_handler.connection_calls, 1);
    assert_eq!(client_handler.connection_failed_calls, 0);
    assert_eq!(client_handler.connection_congestion_state_calls, 0);
    // This is somewhat random and depends on how excatly the two threads
    // interact
    // assert_eq!(client_handler.connection_packet_lost_calls, 0);
    assert_eq!(client_handler.connection_lost_calls, 1);
    assert_eq!(client_handler.connection_closed_calls, 0);

}

#[test]
fn test_server_client_connection_close() {

    // Get a free local socket and then drop it for quick re-use
    // this is not 100% safe but we cannot easily get the locally bound server
    // address after bind() has been called
    let address: Option<net::SocketAddr> = {
        Some(net::UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap())
    };

    // Setup Test Server
    let server_address = address.clone();
    let server_thread = thread::spawn(move|| {

        let config = Config {
            connection_init_threshold: 1000,
            .. Default::default()
        };

        let mut server_handler = MockServerHandler::new(5, true);
        let mut server = Server::new(config);
        server.bind(&mut server_handler, server_address.unwrap()).unwrap();

        assert_eq!(server_handler.connection_lost_calls, 0);
        assert_eq!(server_handler.connection_calls, 1);
        assert_eq!(server_handler.connection_closed_calls, 1);
        assert_eq!(server_handler.closed_by_remote, false);

    });

    let config = Config {
        connection_init_threshold: 1000,
        .. Default::default()
    };

    // TODO cause might be connection marked as dropped by client and
    // an attempted reconnect?
    // client might not receive the closure frame in time?
    //
    // TODO another cause might be that the server thinks the connection is
    // already lost before having send the closure packets? this might explain
    // why the client reconnects afterwards

    let mut client_handler = MockClientHandler::new();
    let mut client = Client::new(config);
    thread::sleep(Duration::from_millis(50));
    client.connect(&mut client_handler, address.unwrap()).unwrap();

    server_thread.join().unwrap();
    assert_eq!(client_handler.connection_lost_calls, 0);
    assert_eq!(client_handler.connection_calls, 1);
    assert_eq!(client_handler.connection_closed_calls, 1);
    assert_eq!(client_handler.closed_by_remote, true);


}

// Client Mock ----------------------------------------------------------------
pub struct MockClientHandler {
    pub connect_calls: u32,
    pub tick_connection_calls: u32,
    pub close_calls: u32,

    pub connection_calls: u32,
    pub connection_failed_calls: u32,
    pub connection_congestion_state_calls: u32,
    pub connection_packet_lost_calls: u32,
    pub connection_lost_calls: u32,
    pub connection_closed_calls: u32,
    pub closed_by_remote: bool
}

impl MockClientHandler {
    pub fn new() -> MockClientHandler {
        MockClientHandler {
            connect_calls: 0,
            tick_connection_calls: 0,
            close_calls: 0,

            connection_calls: 0,
            connection_failed_calls: 0,
            connection_congestion_state_calls: 0,
            connection_packet_lost_calls: 0,
            connection_lost_calls: 0,
            connection_closed_calls: 0,
            closed_by_remote: false
        }
    }
}

impl Handler<Client> for MockClientHandler {

    fn connect(&mut self, _: &mut Client) {
        self.connect_calls += 1;
    }

    fn tick_connection(&mut self, _: &mut Client, _: &mut Connection) {
        self.tick_connection_calls += 1;
    }

    fn close(&mut self, _: &mut Client) {
        self.close_calls += 1;
    }

    fn connection(&mut self, _: &mut Client, _: &mut Connection) {
        self.connection_calls += 1;
    }

    fn connection_failed(&mut self, client: &mut Client, _: &mut Connection) {
        self.connection_failed_calls += 1;
        client.close().unwrap();
    }

    fn connection_packet_lost(
        &mut self, _: &mut Client, _: &mut Connection, _: &[u8]
    ) {
        self.connection_packet_lost_calls += 1;
    }

    fn connection_congestion_state(&mut self, _: &mut Client, _: &mut Connection, _: bool) {
        self.connection_congestion_state_calls += 1;
    }

    fn connection_lost(&mut self, client: &mut Client, _: &mut Connection) {
        self.connection_lost_calls += 1;
        client.close().unwrap();
    }

    fn connection_closed(&mut self, client: &mut Client, _: &mut Connection, by_remote: bool) {
        self.connection_closed_calls += 1;
        self.closed_by_remote = by_remote;
        client.close().unwrap();
    }

}


// Server Mock ----------------------------------------------------------------
pub struct MockServerHandler {

    shutdown_ticks: u32,
    close_connection: bool,

    pub shutdown_calls: u32,
    pub tick_connections_calls: u32,
    pub bind_calls: u32,

    pub connection_calls: u32,
    pub connection_failed_calls: u32,
    pub connection_congestion_state_calls: u32,
    pub connection_packet_lost_calls: u32,
    pub connection_lost_calls: u32,
    pub connection_closed_calls: u32,
    pub closed_by_remote: bool
}

impl MockServerHandler {
    pub fn new(shutdown_ticks: u32, close_connection: bool) -> MockServerHandler {
        MockServerHandler {
            shutdown_ticks: shutdown_ticks,
            close_connection: close_connection,

            shutdown_calls: 0,
            tick_connections_calls: 0,
            bind_calls: 0,

            connection_calls: 0,
            connection_failed_calls: 0,
            connection_congestion_state_calls: 0,
            connection_packet_lost_calls: 0,
            connection_lost_calls: 0,
            connection_closed_calls: 0,
            closed_by_remote: false
        }
    }
}

impl Handler<Server> for MockServerHandler {

    fn bind(&mut self, _: &mut Server) {
        self.bind_calls += 1;
    }

    fn tick_connections(
        &mut self, server: &mut Server,
        connections: &mut HashMap<ConnectionID, Connection>
    ) {

        // Ensure hashmap and connection object have the same id
        for (id, conn) in connections.iter() {
            assert_eq!(*id, conn.id());
        }

        // only advance until initial connection close
        // when there is a connection
        if connections.len() == 0 && self.close_connection {
            self.tick_connections_calls = 0;
        }

        self.tick_connections_calls += 1;

        if self.tick_connections_calls == self.shutdown_ticks {

            // close the client connection the first time around
            if self.close_connection {
                for (_, conn) in connections.iter_mut() {
                    conn.close();
                }

            // shutdown the server
            } else {
                server.shutdown().unwrap();
            }
        }

    }

    fn shutdown(&mut self, _: &mut Server) {
        self.shutdown_calls += 1;
    }

    fn connection(&mut self, _: &mut Server, _: &mut Connection) {
        self.connection_calls += 1;
    }

    fn connection_failed(&mut self, _: &mut Server, _: &mut Connection) {
        self.connection_failed_calls += 1;
    }

    fn connection_packet_lost(
        &mut self, _: &mut Server, _: &mut Connection, _: &[u8]
    ) {
        self.connection_packet_lost_calls += 1;
    }

    fn connection_congestion_state(&mut self, _: &mut Server, _: &mut Connection, _: bool) {
        self.connection_congestion_state_calls += 1;
    }

    fn connection_lost(&mut self, _: &mut Server, _: &mut Connection) {
        self.connection_lost_calls += 1;
    }

    fn connection_closed(&mut self, _: &mut Server, _: &mut Connection, by_remote: bool) {
        self.connection_closed_calls += 1;
        self.closed_by_remote = by_remote;
        self.tick_connections_calls = 0;
        self.shutdown_ticks = 5;
        self.close_connection = false;
    }

}

