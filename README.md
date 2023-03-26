## Sunlight

A tidy little virtual machine room service. Mostly written in Rust, with the frontend written in TypeScript.

## Setup/hosting

TODO (wait!)

## Access control

TODO. for now, just only share links with people you trust


## Architehcure

The source tree is a cargo workspace with multiple crates.

```
sunlight (sunlight_) -> Main server, 
		vm (sunlight_vm) -> VM library. Runs QEMU virtual machines with vGPU devices
						(and when possible, GPU-accelerated encoding).
```
