use super::enums::*;
use tokio::process::*;

pub(crate) fn bool_to_qemu(val: bool) -> String {
	if val {
		return String::from("on");
	}
	return String::from("off");
}

/// trait that objects which want to convert to QEMU options implement
pub trait QemuOption {

	/// Returns QEMU arguments 
	fn as_options(&self) -> String;

	/// Validate that the options generated will actually work. The base implementation
	/// provided inside the trait definition is good enough for most cases, but anything
	/// more complex will need more complex validation logic.
	fn validate(&self, _machine: &VirtualMachine) -> bool {
		true
	}

}


pub enum MachineType {
	/// PC machine type. Uses a i440fx chipset.
	Pc {
		acpi: bool,
		usb: bool
	},

	/// Q35 machine type. Preferrable for VMs running more modern guests, and required for vGPU support.
	Q35 {
		acpi: bool,
		usb: bool,
		hmat: bool
	}
}

pub struct Cpu {
	/// The CPU model.
	pub model : String,

	/// CPU features. Later on, these can be typed/exclusions.
	/// For now, I don't care.
	pub features : Vec<String>,
	
	pub core_count: i8
}

pub struct Memory {
	pub size: String,
	pub prealloc: bool
}

pub enum Snapshot {
	NoSnapshots,

	/// This uses the QEMU `-snapshot` option. This creates a temporary
	/// backing file in /var/tmp, where all data differences will be stored 
	/// (and deleted when the VM shuts down.)
	HdSnapshot,

}

pub enum DiskInterface {
	/// IDE (or SATA if using the q35 machine type.)
	Ide, 

	/// SCSI (incl. VirtIO SCSI).
	Scsi
}


pub enum DiskDrive {
	CdDrive {
		interface: DiskInterface,
		id: String
	},

	HdDrive {
		id: String,
		interface: DiskInterface,
		image_path: String,
		readonly: bool,
		format: String,
		ssd: bool,
		cache: Option<String>, // will be omitted if None
		aio: Option<String>
	},

	/// A pflash drive. There are no configurable interface types.
	Pflash {
		id: String,
		image_path: String,
		readonly: bool,
		format: String
	}

}

pub enum DiskController {
	VirtioScsi {
		id: String
	}
}

pub enum GraphicsAdapter {
	/// Standard VGA adapter.
	StdVga {
		ram_size_mb: i16
	},

	/// Cirrus Logic GD5446.
	CirrusVga {
		ram_size_mb: i16
	},

	/// Red Hat QXL.
	QxlVga {},

	/// A Mediated Device (MDEV) vGPU device, provided by supported GPU devices. 
	/// 
	/// Currently this means/supports:
	/// - Intel GVT-g
	/// - NVIDIA vGPU
	VgpuVga {
		/// The MDEV UUID. This *must* match the VM's UUID, at least for NVIDIA. 
		/// I don't know if a similar requirement is true for Intel, but I assume it is.
		uuid: String,

		/// Use the QEMU ramfb device to provide pre-boot (pre-graphics driver initalization) video.
		use_ramfb: bool,

		// these are used for bypassing vgpu driver restrictions
		
		pci_vendor_id: Option<String>,
		pci_device_id: Option<String>,
		pci_sub_vendor_id: Option<String>,
		pci_sub_device_id: Option<String>		
	}

}

pub enum Network {
	User {
		id: String
	},

	Tap {
		id: String,
		dev: String
	}
}

pub enum NetworkAdapter {
	Virtio {
		id: String,
		netdev: String,
		mac: Option<String>
	},
	
	Rtl8139 {
		id: String,
		netdev: String,
		mac: Option<String>
	},

	// more variants?

}

impl QemuOption for MachineType {
	fn as_options(&self) -> String {
		match self {
			Self::Pc { acpi, usb } => format!("-machine pc,acpi={},usb={}", bool_to_qemu(*acpi), bool_to_qemu(*usb)),
			Self::Q35 { acpi, usb, hmat } => format!("-machine q35,acpi={},usb={},hmat={} -device ioh3420,id=vm.pcie_root,slot=0,bus=pcie.0", bool_to_qemu(*acpi), bool_to_qemu(*usb), bool_to_qemu(*hmat)),
			//_ => panic!("Unhandled machine type in MachineType::as_options()")
		}
	}
}

impl QemuOption for Cpu {
	fn as_options(&self) -> String {
		// Make sure there are features for us to append
		if self.features.is_empty() {
			format!("-cpu {} -smp cores={}", self.model, self.core_count)
		} else {
			format!("-cpu {},{} -smp cores={}", self.model, self.features.join(","), self.core_count)
		}
	}

	fn validate(&self, _machine: &VirtualMachine) -> bool {
		// should probably also check features, but it IS ok for that to be empty
		// we explicitly check for it when doing as_options() at least
		!self.model.is_empty()
	}
}

impl QemuOption for Memory {
	fn as_options(&self) -> String {
		// TODO: we should allow memory backends, because -mem-prealloc is self-deprecated
		if self.prealloc {
			return format!("-m {} -mem-prealloc", self.size);
		}

		return format!("-m {}", self.size);
	}
}


impl QemuOption for DiskController {
	fn as_options(&self) -> String {
		match self {
			Self::VirtioScsi { id } => format!("-object iothread,id=vm.{id}.block_thread -device virtio-scsi-pci,num_queues=6,iothread=vm.{id}.block_thread,id=vm.{id}")
		}
	}
}

impl QemuOption for DiskDrive {

	fn as_options(&self) -> String {
		match self {
			Self::CdDrive { interface, id } => {
				match interface {
					DiskInterface::Ide => {
						format!("-drive if=none,media=cdrom,aio=io_uring,id={id} -device ide-cd,drive={id},id={id}.drive")
					},
					DiskInterface::Scsi => {
						format!("-drive if=none,media=cdrom,aio=io_uring,id={id} -device scsi-cd,drive={id},id={id}.drive")
					}
				}
			},
			Self::HdDrive { id, interface, image_path, readonly, format, ssd, cache, aio } => {
				let mut drive_str = format!("-drive if=none,file={image_path},format={format},id=vm.{id}.drive,readonly={}", bool_to_qemu(*readonly));


				match cache {
					Some(str) => drive_str.push_str(format!(",cache={str}").as_str()),
					_ => {}
				}

				match aio {
					Some(str) => drive_str.push_str(format!(",aio={str}").as_str()),
					_ => {}
				}

				let mut opts_str = format!("id=vm.{id},drive=vm.{id}.drive");

				// if on an ssd
				if *ssd {
					opts_str.push_str(",rotation_rate=1");
				}

				match interface {
					DiskInterface::Ide => format!("{drive_str} -device ide-hd,{opts_str}"),
					DiskInterface::Scsi => format!("{drive_str} -device scsi-hd,{opts_str}")
				}
			}

			_ => panic!("a certified stupid flower moment probably")
		}

	}
}

impl QemuOption for GraphicsAdapter {
	fn as_options(&self) -> String {
		match self {
			Self::StdVga { ram_size_mb } => format!("-device VGA,vgamem_mb={},id=vm.vga", ram_size_mb),
			Self::CirrusVga { ram_size_mb } => format!("-device cirrus-vga,vgamem_mb={},id=vm.vga", ram_size_mb),
			Self::QxlVga {  } => format!("-device qxl-vga,id=vm.vga"),
			Self::VgpuVga { uuid, use_ramfb, pci_vendor_id, pci_device_id, pci_sub_vendor_id, pci_sub_device_id } => {
				let path = format!("/sys/bus/mdev/devices/{uuid}");
				if pci_vendor_id.is_some() {
					let vid = pci_device_id.as_deref().unwrap();
					let pid = pci_vendor_id.as_deref().unwrap();
					let subvid = pci_sub_vendor_id.as_deref().unwrap();
					let subpid = pci_sub_device_id.as_deref().unwrap();
					return format!("-device vfio-pci-nohotplug,sysfsdev={path},display=on,ramfb={},id=vm.vgpu,bus=vm.pcie_root,addr=0x0,x-pci-vendor-id={vid},x-pci-device-id={pid},x-pci-sub-vendor-id={subvid},x-pci-sub-device-id={subpid}", bool_to_qemu(*use_ramfb));
				}
				return format!("-device vfio-pci-nohotplug,sysfsdev={path},display=on,ramfb={},id=vm.vgpu,bus=vm.pcie_root,addr=0x0", bool_to_qemu(*use_ramfb));
			}
		}
	}

	fn validate(&self, machine: &VirtualMachine) -> bool {
		match self {
			Self::VgpuVga { uuid, .. } => { 
				// if the machine doesn't even *have* a uuid, 
				// it's probably not configured properly
				if machine.uuid.is_none() {
					return false;
				}

				match machine.machine {
					Some(MachineType::Q35 { .. }) => true,

					// vGPU can't be used in a PC configuration or an invalid one
					Some(MachineType::Pc { .. }) => false,
					None => false
				};

				// likewise, if we don't have one, then we're
				// the misconfigured one
				if uuid.is_empty() {
					return false
				}

				return machine.uuid.as_deref().unwrap() == uuid;
			}

			_ => true // no special cases
		}
	}
}

impl QemuOption for Network {
	fn as_options(&self) -> String {	
		match self {
			Self::User { id } => format!("-netdev user,id=vm.{id}"),
			Self::Tap { id, dev } => format!("-netdev tap,vhost=on,script=no,downscript=no,ifname={dev},id=vm.{id}")
		}
	}
}

impl QemuOption for NetworkAdapter {
	fn as_options(&self) -> String {
		match self {
			Self::Virtio { id, netdev, mac } => {
				let mut base = format!("-device virtio-net-pci,id=vm.{id},netdev=vm.{netdev}");
				match mac {
					Some(addr) => base.push_str(format!(",mac={addr}").as_str()),
					_ => {}
				}
				base
			},

			Self::Rtl8139 { id, netdev, mac } => {
				let mut base = format!("-device rtl8139,id=vm.{id},netdev=vm.{netdev}");
				match mac {
					Some(addr) => base.push_str(format!(",mac={addr}").as_str()),
					_ => {}
				}
				base
			}
		}
	}
}


fn join_options<'a>(vec: &'a Vec<Box<dyn QemuOption + 'a>>, machine: &VirtualMachine) -> Vec<String> {
	// this is occursed
	vec.iter()
		.map(|o| {
			if o.validate(machine) {
				o.as_options()
			} else {
				String::from("uh oh system fuck")
			}
		}).collect::<Vec<String>>()
		//.join(" ")
}

/// A QEMU virtual machine.
pub struct VirtualMachine<'a> {
	// process
	process: Option<Command>,

	name: String,
	uuid: Option<String>,
	machine: Option<MachineType>,
	devices: Vec<Box<dyn QemuOption + 'a>>,
	drives: Vec<Box<dyn QemuOption + 'a>>

}


impl<'a> VirtualMachine<'a> {
	pub fn new(name: &str) -> Result<VirtualMachine<'a>, VMCreateError> {
		let name_str = String::from(name);

		if name_str.contains(' ') {
			// stop that!
			Err(VMCreateError::InvalidName)
		} else {
			Ok(VirtualMachine {
				process: None,
				name: name_str,
				uuid: None,
				machine: None,
				devices: Vec::new(),
				drives: Vec::new()
			})
		}
	}

	/// Set the name of this VM.
	pub fn set_name(&mut self, name: &str) -> &mut VirtualMachine<'a> {
		self.name = String::from(name);
		self
	}

	/// Set the UUID of this VM.
	pub fn set_uuid(&mut self, uuid: &str) -> &mut VirtualMachine<'a> {
		self.uuid = Some(String::from(uuid));
		self
	}

	pub fn set_machine_type(&mut self, machine: MachineType) -> &mut VirtualMachine<'a> {
		self.machine = Some(machine);
		self
	}

	/// Add something which implements the Options trait to this VM.
	pub fn add_device<T: QemuOption + 'a>(&mut self, dev: T) -> &mut VirtualMachine<'a> {
		self.devices.push(Box::new(dev));
		self
	}

	pub fn add_drive<T: QemuOption + 'a>(&mut self, dev: T) -> &mut VirtualMachine<'a> {
		self.drives.push(Box::new(dev));
		self
	}

	/// Generate the QEMU command arguments that will be used to run this VM. This includes some options
	/// which are always generated, to aid Sunlight's out-of-band management of the VM.
	pub fn to_arguments(&self) -> Result<Vec<String>, VMQemuProcessStartError> {

		if self.machine.is_none() {
			return Err(VMQemuProcessStartError::NoMachineType);
		}

		let mut vec = vec![
			String::from("-nodefaults"),
			String::from("-accel kvm"),
		];

		vec.push(format!("-name {},process=sunlight_{}", self.name, self.name));
		vec.push(self.machine.as_ref().unwrap().as_options());
		vec.append(&mut join_options(&self.devices, self));
		vec.append(&mut join_options(&self.drives, self));

		Ok(vec)

		/*Ok(format!("-nodefaults -accel kvm -name {},process=sunlight_{} {} {} {}",
			self.name,
			self.name,
			self.machine.as_ref().unwrap().as_options(),
			join_options(&self.devices, &self),
			join_options(&self.drives, &self)))*/
	}

	pub  fn start(&mut self) -> Result<(), VMQemuProcessStartError> {
		//self.process = Some(Command::new("qemu-system-x86_64"));

		let args = match self.to_arguments() {
			Ok(_args) => _args,
			Err(..) => return Err(VMQemuProcessStartError::ErrorBuildingCommandLine)
		};

		println!("{:#?}", args);

		Ok(())
	}
}
