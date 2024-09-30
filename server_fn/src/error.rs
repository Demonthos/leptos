use std::{
    fmt::{Display, Write},
    str::FromStr,
};
use thiserror::Error;
use url::Url;

/// A custom header that can be used to indicate a server function returned an error.
pub const SERVER_FN_ERROR_HEADER: &str = "serverfnerror";

/// Wraps some error type, which may implement any of [`Error`](trait@std::error::Error), [`Clone`], or
/// [`Display`].
#[derive(Debug)]
pub struct WrapError<T>(pub T);

/// A helper macro to convert a variety of different types into `ServerFnErrorErr`.
/// This should mostly be used if you are implementing `From<ServerFnErrorErr>` for `YourError`.
#[macro_export]
macro_rules! server_fn_error {
    () => {{
        use $crate::{ViaError, WrapError};
        (&&WrapError(())).to_server_error()
    }};
    ($err:expr) => {{
        use $crate::error::{ViaError, WrapError};
        match $err {
            error => (&&WrapError(error)).to_server_error(),
        }
    }};
}

/// This trait serves as the conversion method between a variety of types
/// and [`ServerFnErrorErr`].
pub trait ViaError {
    /// Converts something into an error.
    fn to_server_error(&self) -> ServerFnErrorErr;
}

// This impl should catch if you fed it a [`ServerFnErrorErr`] already.
impl ViaError for &&&&WrapError<ServerFnErrorErr> {
    fn to_server_error(&self) -> ServerFnErrorErr {
        self.0.clone()
    }
}

// If it doesn't impl Error, but does impl Display and Clone,
// we can still wrap it in String form
impl<E: Display + Clone> ViaError for &WrapError<E> {
    fn to_server_error(&self) -> ServerFnErrorErr {
        ServerFnErrorErr::ServerError(self.0.to_string())
    }
}

// This is what happens if someone tries to pass in something that does
// not meet the above criteria
impl<E> ViaError for WrapError<E> {
    #[track_caller]
    fn to_server_error(&self) -> ServerFnErrorErr {
        panic!(
            "At {}, you call `to_server_error()` or use  `server_fn_error!` \
             with a value that does not implement `Clone` and either `Error` \
             or `Display`.",
            std::panic::Location::caller()
        );
    }
}

// /// Type for errors that can occur when using server functions.
// ///
// /// Unlike [`ServerFnErrorErr`], this does not implement [`Error`](trait@std::error::Error).
// /// This means that other error types can easily be converted into it using the
// /// `?` operator.
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(
//     feature = "rkyv",
//     derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
// )]
// pub enum ServerFnErrorErr {
//     /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
//     Registration(String),
//     /// Occurs on the client if there is a network error while trying to run function on server.
//     Request(String),
//     /// Occurs on the server if there is an error creating an HTTP response.
//     Response(String),
//     /// Occurs when there is an error while actually running the function on the server.
//     ServerError(String),
//     /// Occurs on the client if there is an error deserializing the server's response.
//     Deserialization(String),
//     /// Occurs on the client if there is an error serializing the server function arguments.
//     Serialization(String),
//     /// Occurs on the server if there is an error deserializing one of the arguments that's been sent.
//     Args(String),
//     /// Occurs on the server if there's a missing argument.
//     MissingArg(String),
// }

// impl ServerFnErrorErr {
//     /// Constructs a new [`ServerFnErrorErr::ServerError`] from some other type.
//     pub fn new(msg: impl ToString) -> Self {
//         Self::ServerError(msg.to_string())
//     }
// }

// impl<E: std::error::Error> From<E> for ServerFnErrorErr {
//     fn from(value: E) -> Self {
//         ServerFnErrorErr::ServerError(value.to_string())
//     }
// }

// impl Display for ServerFnErrorErr {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "{}",
//             match self {
//                 ServerFnErrorErr::Registration(s) => format!(
//                     "error while trying to register the server function: {s}"
//                 ),
//                 ServerFnErrorErr::Request(s) => format!(
//                     "error reaching server to call server function: {s}"
//                 ),
//                 ServerFnErrorErr::ServerError(s) =>
//                     format!("error running server function: {s}"),
//                 ServerFnErrorErr::Deserialization(s) =>
//                     format!("error deserializing server function results: {s}"),
//                 ServerFnErrorErr::Serialization(s) =>
//                     format!("error serializing server function arguments: {s}"),
//                 ServerFnErrorErr::Args(s) => format!(
//                     "error deserializing server function arguments: {s}"
//                 ),
//                 ServerFnErrorErr::MissingArg(s) => format!("missing argument {s}"),
//                 ServerFnErrorErr::Response(s) =>
//                     format!("error generating HTTP response: {s}"),
//             }
//         )
//     }
// }

/// A trait that all custom server function error types must implement.
/// Custom error types must:
/// - implement [`From<ServerFnErrorErr>`] so that they can be converted from [`ServerFnErrorErr`]
/// - implement [`FromStr`] and [`Display`] so that they can be converted from and to strings
/// - implement [`ServerFnErrorSerde`] so that they can be serialized and deserialized
pub trait CustomServerFnError:
    ServerFnErrorSerde + From<ServerFnErrorErr> + FromStr + Display
{
}

/// A serializable custom server function error type.
///
/// This is implemented for all types that implement [`FromStr`] + [`Display`].
///
/// This means you do not necessarily need the overhead of `serde` for a custom error type.
/// Instead, you can use something like `strum` to derive `FromStr` and `Display` for your
/// custom error type.
///
/// This is implemented for the default [`ServerFnErrorErr`].
pub trait ServerFnErrorSerde: Sized {
    /// Converts the custom error type to a [`String`].
    fn ser(&self) -> Result<String, std::fmt::Error>;

    /// Deserializes the custom error type from a [`String`].
    fn de(data: &str) -> Self;
}

impl ServerFnErrorSerde for ServerFnErrorErr {
    fn ser(&self) -> Result<String, std::fmt::Error> {
        let mut buf = String::new();
        match self {
            Self::Registration(e) => {
                write!(&mut buf, "Registration|{e}")
            }
            Self::Request(e) => write!(&mut buf, "Request|{e}"),
            Self::Response(e) => write!(&mut buf, "Response|{e}"),
            Self::Deserialization(e) => {
                write!(&mut buf, "Deserialization|{e}")
            }
            Self::Serialization(e) => {
                write!(&mut buf, "Serialization|{e}")
            }
            Self::Args(e) => write!(&mut buf, "Args|{e}"),
            Self::MissingArg(e) => {
                write!(&mut buf, "MissingArg|{e}")
            }
        }?;
        Ok(buf)
    }

    fn de(data: &str) -> Self {
        data.split_once('|')
            .and_then(|(ty, data)| match ty {
                "Registration" => {
                    Some(Self::Registration(data.to_string()))
                }
                "Request" => Some(Self::Request(data.to_string())),
                "Response" => Some(Self::Response(data.to_string())),
                "Deserialization" => {
                    Some(Self::Deserialization(data.to_string()))
                }
                "Serialization" => {
                    Some(Self::Serialization(data.to_string()))
                }
                "Args" => Some(Self::Args(data.to_string())),
                "MissingArg" => {
                    Some(Self::MissingArg(data.to_string()))
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                Self::Deserialization(format!(
                    "Could not deserialize error {data:?}"
                ))
            })
    }
}

/// Type for errors that can occur when using server functions.
///
/// Unlike [`ServerFnErrorErr`], this implements [`std::error::Error`]. This means
/// it can be used in situations in which the `Error` trait is required, but it’s
/// not possible to create a blanket implementation that converts other errors into
/// this type.
///
/// [`ServerFnErrorErr`] and [`ServerFnErrorErr`] mutually implement [`From`], so
/// it is easy to convert between the two types.
#[derive(Error, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub enum ServerFnErrorErr {
    /// Error while trying to register the server function (only occurs in case of poisoned RwLock).
    #[error("error while trying to register the server function: {0}")]
    Registration(String),
    /// Occurs on the client if there is a network error while trying to run function on server.
    #[error("error reaching server to call server function: {0}")]
    Request(String),
    /// Occurs on the client if there is an error deserializing the server's response.
    #[error("error deserializing server function results: {0}")]
    Deserialization(String),
    /// Occurs on the client if there is an error serializing the server function arguments.
    #[error("error serializing server function arguments: {0}")]
    Serialization(String),
    /// Occurs on the server if there is an error deserializing one of the arguments that's been sent.
    #[error("error deserializing server function arguments: {0}")]
    Args(String),
    /// Occurs on the server if there's a missing argument.
    #[error("missing argument {0}")]
    MissingArg(String),
    /// Occurs on the server if there is an error creating an HTTP response.
    #[error("error creating response {0}")]
    Response(String),
}

/// Associates a particular server function error with the server function
/// found at a particular path.
///
/// This can be used to pass an error from the server back to the client
/// without JavaScript/WASM supported, by encoding it in the URL as a query string.
/// This is useful for progressive enhancement.
#[derive(Debug)]
pub struct ServerFnUrlError<CustErr = ServerFnErrorErr> {
    path: String,
    error: CustErr,
}

impl<CustErr> ServerFnUrlError<CustErr> {
    /// Creates a new structure associating the server function at some path
    /// with a particular error.
    pub fn new(path: impl Display, error: CustErr) -> Self {
        Self {
            path: path.to_string(),
            error,
        }
    }

    /// The error itself.
    pub fn error(&self) -> &CustErr {
        &self.error
    }

    /// The path of the server function that generated this error.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Adds an encoded form of this server function error to the given base URL.
    pub fn to_url(&self, base: &str) -> Result<Url, url::ParseError>
    where
        CustErr: CustomServerFnError,
    {
        let mut url = Url::parse(base)?;
        url.query_pairs_mut()
            .append_pair("__path", &self.path)
            .append_pair(
                "__err",
                &ServerFnErrorSerde::ser(&self.error).unwrap_or_default(),
            );
        Ok(url)
    }
}

impl ServerFnUrlError {
    /// Replaces any ServerFnUrlError info from the URL in the given string
    /// with the serialized success value given.
    pub fn strip_error_info(path: &mut String) {
        if let Ok(mut url) = Url::parse(&*path) {
            // NOTE: This is gross, but the Serializer you get from
            // .query_pairs_mut() isn't an Iterator so you can't just .retain().
            let pairs_previously = url
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<Vec<_>>();
            let mut pairs = url.query_pairs_mut();
            pairs.clear();
            for (key, value) in pairs_previously
                .into_iter()
                .filter(|(key, _)| key != "__path" && key != "__err")
            {
                pairs.append_pair(&key, &value);
            }
            drop(pairs);
            *path = url.to_string();
        }
    }
}

impl From<ServerFnUrlError> for ServerFnErrorErr {
    fn from(error: ServerFnUrlError) -> Self {
        error.error
    }
}
