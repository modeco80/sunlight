use sunlight_vm::qemu::vm::*;

fn main() {
	let mut vm = VirtualMachine::new("test")
		.expect("should be valid VM");

	// build up the VM
	vm.set_machine_type(MachineType::Q35 { acpi: true, usb: true, hmat: false })
		.add_device(Cpu {
			model: String::from("host"),
			features: vec![],
			core_count: 2
		})
		.add_device(Memory { size: String::from("4G"), prealloc: true })
		.add_device(GraphicsAdapter::StdVga { ram_size_mb: 8 })
		.add_device(DiskController::VirtioScsi { id: String::from("scsic") })
		.add_device(Network::User { id: String::from("usernet") })
		.add_device(NetworkAdapter::Virtio { id: String::from("net0"), netdev: String::from("usernet"), mac: None })
		.add_drive(DiskDrive::CdDrive { interface: DiskInterface::Scsi, id: String::from("cd") })
		.add_drive(DiskDrive::HdDrive { 
			id: String::from("sdda"), 
			interface: DiskInterface::Scsi, 
			image_path: String::from("/home/lily/test.qcow2"), 
			readonly: false, 
			format: String::from("qcow2"), 
			ssd: true, 
			cache: Some(String::from("writethrough")),
			aio: Some(String::from("io_uring")) 
		});

	vm.start();

	//println!("{}", vm.to_command().expect("this should work lol"));
}
