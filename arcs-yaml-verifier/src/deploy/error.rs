use std::fmt::Display;

use crate::structs::ValueType;

#[derive(Debug, Clone, PartialEq)]
pub enum ExposeError {
    Missing,
    BadFormat(String),
    BadParts {
        data: String,
        port: bool,
        protocol: bool,
    }
}
impl Display for ExposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => write!(f, "You must specify what port and protocol to expose."),
            Self::BadFormat(s) => write!(f, "The `expose` value must follos the format <port>/<protocol>. `{s}` does not match. (Protocols are `udp` and `tcp`)"),
            Self::BadParts {
                data,
                port,
                protocol,
            } => if *port && *protocol {
                write!(f, "The port must be a number and the protocol must be `tcp` or `udp`. ({data} was recieved)")
            } else if *port {
                write!(f, "The port must be a number. ({data} was recieved)")
            } else {
                write!(f, "The protocol must be `tcp` or `udp`. ({data} was recieved)")
            },
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentTargetOptionsError {
    BadBaseType(ValueType),
    Parts {
        expose: Option<ExposeError>,
        replicas_invalid: Option<ValueType>,    
    }
}
impl Display for DeploymentTargetOptionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadBaseType(t) => writeln!(f, "A target should be a map with values for `expose` and (optionally) `replicas`, not {t}."),
            Self::Parts {
                expose,
                replicas_invalid
            } => {
                writeln!(f, "There were issues with certain parts of this target:")?;
                if let Some(expose_error) = expose {
                    writeln!(f, "            {expose_error}")?;
                }
                if let Some(invalid_type) = replicas_invalid {
                    writeln!(f, "            `replicas` should be a number from 1 - 255, not {invalid_type}.")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeployOptionsError {
    Parts {
        web: Option<DeploymentTargetOptionsError>,
        admin: Option<DeploymentTargetOptionsError>,
        nc: Option<DeploymentTargetOptionsError>,
    },
    BadBaseType(ValueType),
}
impl Display for DeployOptionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadBaseType(t) => writeln!(f, "Deploy should be a map of `web`, `admin`, and `nc`, not {t}."),
            Self::Parts {
                web,
                admin,
                nc
            } => {
                writeln!(f, "There were issues with certain deployment targets:")?;
                if let Some(web  ) = web   { writeln!(f, "        web:   {web}")?;   }
                if let Some(admin) = admin { writeln!(f, "        admin: {admin}")?; }
                if let Some(nc   ) = nc    { writeln!(f, "        nc:    {nc}")?;    }
                Ok(())
            }
        }
    }
}
