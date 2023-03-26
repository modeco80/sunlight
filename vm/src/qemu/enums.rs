use thiserror::Error;

#[derive(Error, Debug)]
pub enum VMQemuProcessStartError {
	#[error("no QEMU machine type specified")]
	NoMachineType,

	#[error("error building QEMU command line from devices")]
	ErrorBuildingCommandLine,

	#[error(transparent)]
	IoError(#[from] std::io::Error)
}

#[derive(Error, Debug)]
pub enum VMQmpConnectionError {
	#[error(transparent)]
	IoError(#[from] std::io::Error)
}

#[derive(Error, Debug)]
pub enum VMQmpHandshakeError {
	#[error(transparent)]
	IoError(#[from] std::io::Error)
}

#[derive(Error, Debug)]
pub enum VMDbusConnectionError {
	#[error(transparent)]
	IoError(#[from] std::io::Error)
}

#[derive(Error, Debug)]
pub enum VMStartError {
	/// An error occured while attempting to start the QEMU process.
	/// (typically QEMU exiting with a non-zero exit code.)
	/// (attach tokio process error here)
	#[error("failure starting QEMU process")]
	QemuProcessStartFailure(#[from] VMQemuProcessStartError),

	/// There was an error connecting to QMP (required to begin the p2p D-Bus handshake).
	#[error("failure connecting to QMP")]
	QmpConnectionFailure(#[from] VMQmpConnectionError),

	/// There was an error handshaking QMP with QEMU.
	/// (attach qapi crate error here)
	#[error("failure handshaking with QMP server")]
	QmpHandshakeFailure(#[from] VMQmpHandshakeError),

	/// There was an error starting the p2p D-Bus session between QEMU and Sunlight.
	/// (ditto, but with zbus errors? or would it be more worth it to box a value here?)
	#[error("failure initiating p2p D-Bus connection")]
	DbusConnectionFailure(#[from] VMDbusConnectionError)
}

#[derive(Error, Debug)]
pub enum VMCreateError {

	#[error("invalid name characters pressent")]
	InvalidName,

}

/// Current VM state.
#[derive(Debug, Clone)]
pub enum VMState {
	/// The VM is not running.
	Stopped,

	/// The VM is attempting to start.
	Starting,

	/// The VM is running.
	Started,

	/// The VM is atttempting to stop.
	Stopping
}

