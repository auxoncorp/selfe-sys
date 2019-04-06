*** Build for x86
```
      cargo build
```
    
*** Build for arm

```
      cd libsel4-sys-gen
      SEL4_PLATFORM=sabre xargo build --target=armv7-unknown-linux-gnueabihf -vv
```
