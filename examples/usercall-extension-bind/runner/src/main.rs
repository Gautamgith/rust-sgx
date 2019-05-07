/* Copyright (c) Fortanix, Inc.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate aesm_client;
extern crate enclave_runner;
extern crate sgxs_loaders;

use aesm_client::AesmClient;
use enclave_runner::usercalls::{SyncListener, SyncStream, UsercallExtension};
use enclave_runner::EnclaveBuilder;
use sgxs_loaders::isgx::Device as IsgxDevice;

use std::io::{Result as IoResult, Error, ErrorKind};

use std::net::{TcpListener, TcpStream};

/// This example demonstrates use of usercall extensions for bind call.
/// User call extension allow the enclave code to "bind" to an external service via a customized enclave runner.
/// Here we customize the runner to intercept calls to bind to an address and advance the stream before returning it to enclave
/// This can be usefull to strip protocol encapsulations, say while servicing requests load balanced by HA Proxy.
struct StreamAdvancer {
    listener: TcpListener,
    skip_length : usize,
    pub local_address : String,
}
impl StreamAdvancer {
    fn advance_stream(&self, stream: TcpStream) -> Result<TcpStream, Error> {
        let mut buf : Vec<u8> = vec![0; self.skip_length];
        let read = stream.read(&mut buf)?;
        if read <= self.skip_length {
            Err(ErrorKind::UnexpectedEof.into())
        } else {
            Ok(stream)
        }
    }
    fn new(addr : &str, skip_length : usize) -> Result<StreamAdvancer, Error>{
            TcpListener::bind(addr)
            .map(|listener| {

                                 let local_address = match listener.local_addr() {
                                                     Ok(local) => local.to_string(),
                                                     Err(_) => "error".to_string(),
                                 };
                                 StreamAdvancer { listener,
                                                  skip_length,
                                                  local_address
                                 }
                             })
    }
}

impl SyncListener for StreamAdvancer {
    fn accept(&self) -> IoResult<(Box<SyncStream>, Box<ToString>, Box<ToString>)> {
        let (stream, peer) = self.listener.accept()?;
        let local = match stream.local_addr() {
                Ok(local) => local.to_string(),
                Err(_) => "error".to_string(),
        };
        let stream = self.advance_stream(stream)?;
        Ok((Box::new(stream), Box::new(local), Box::new(peer)))
    }
}


#[derive(Debug)]
struct ExternalService;
// Ignoring local_addr and peer_addr, as they are not relavent in the current context.
impl UsercallExtension for ExternalService {
    fn bind_stream(&self,
                  addr: &str,
                  local_addr: Option<&mut String>
                  ) -> IoResult<Option<Box<SyncListener>>> {
        if addr == "localhost::6010" {
            let advancer = StreamAdvancer::new(addr, 36)?;
            if let Some(local_addr) = local_addr {
                (*local_addr) = advancer.local_address.clone();
            }

            Ok(Some(Box::new(advancer)))
        } else {
            Ok(None)
        }
    }
}

fn usage(name: String) {
    println!("Usage:\n{} <path_to_sgxs_file>", name);
}

fn parse_args() -> Result<String, ()> {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        2 => Ok(args[1].to_owned()),
        _ => {
            usage(args[0].to_owned());
            Err(())
        }
    }
}

fn main() {
    let file = parse_args().unwrap();

    let mut device = IsgxDevice::new()
        .unwrap()
        .einittoken_provider(AesmClient::new())
        .build();

    let mut enclave_builder = EnclaveBuilder::new(file.as_ref());
    enclave_builder.dummy_signature();
    enclave_builder.usercall_extension(ExternalService);
    let enclave = enclave_builder.build(&mut device).unwrap();

    enclave
        .run()
        .map_err(|e| {
            println!("Error while executing SGX enclave.\n{}", e);
            std::process::exit(1)
        })
        .unwrap();
}
