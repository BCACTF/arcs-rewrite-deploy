use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize)]
pub struct YamlFile {
    pub deploy: Deploy,
}

#[derive(Deserialize)]
pub struct Deploy {
    pub web: Option<ChallengeParams>, 
    pub admin: Option<ChallengeParams>,
    pub nc: Option<ChallengeParams>,
}



#[derive(Deserialize)]
pub struct ChallengeParams {
    #[serde(deserialize_with = "implementation_of_deserialize_for_network_protocol::deserialize")]
    pub expose : NetworkProtocol,

    #[serde(default = "default_replicas")]
    pub replicas : u8
}

pub enum NetworkProtocol {
    Tcp(u32),
    Udp(u32),
}

#[doc(hidden)]
mod implementation_of_deserialize_for_network_protocol {
    use super::NetworkProtocol;
    use serde::de::{Visitor, Deserializer};

    pub fn deserialize<'a, D: Deserializer<'a>>(deserializer: D) -> Result<NetworkProtocol, D::Error> {
        struct NetworkProtocolDeserializeVisitor;

        impl<'de> Visitor<'de> for NetworkProtocolDeserializeVisitor {
            type Value = super::NetworkProtocol;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "an network protocol string in the format of port/protocol")
            }
            fn visit_str<E>(self, network_protocol_string: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error, {
                
                let (port, protocol) = network_protocol_string
                    .split_once('/')
                    .ok_or(NetProtDeErr::NoSlash.custom())?;
                
                let port: u32 = port
                    .parse()
                    .map_err(|_| NetProtDeErr::BadPort(port.to_string()).custom())?;
                
                let output_struct = match protocol {
                    "tcp" => Self::Value::Tcp(port),
                    "udp" => Self::Value::Udp(port),
                    _ => return Err(NetProtDeErr::BadProtocol(protocol.to_string()).custom()),
                };

                Ok(output_struct)
            }
        }

        deserializer.deserialize_str(NetworkProtocolDeserializeVisitor)
    }

    use net_prot_de_err::NetProtDeErr as NetProtDeErr;


    pub mod net_prot_de_err {
        use serde::de::StdError;
        use std::fmt::Display;
        use serde::de::Error;

        #[derive(Debug)]
        pub enum NetProtDeErr {
            NoSlash,
            BadPort(String),
            BadProtocol(String),
        }

        impl NetProtDeErr {
            pub fn custom<T: Error>(self) -> T {
                Error::custom(self)
            }
        }

        impl Display for NetProtDeErr {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                use NetProtDeErr::*;
                match self {
                    NoSlash => write!(f, "No slash was found in the string representing the network protocol!"),
                    BadPort(port_str) => write!(f, "{} is not a valid port number!", port_str),
                    BadProtocol(protocol_str) => write!(f, "{} is not a valid protocol identifier!", protocol_str),
                }
            }
        }
        impl StdError for NetProtDeErr {}
        impl Error for NetProtDeErr {
            fn custom<T>(_:T) -> Self where T:Display {
                Self::NoSlash
            }
        }
    }
    
    
}


impl Default for NetworkProtocol {
    fn default() -> Self {
        NetworkProtocol::Tcp(8080)
    }
}
impl Display for NetworkProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkProtocol::Tcp(port) => write!(f, "{}/tcp", port),
            NetworkProtocol::Udp(port) => write!(f, "{}/udp", port)
        }
    }
}

fn default_replicas() -> u8 {
    1
}
