set -e

cd example
xargo build --target=x86_64-unknown-linux-gnu
cd ..

mkdir kernel-build
cd kernel-build

export KERNEL=/home/mullr/devel/auxon-sel4
export SEL4_TOOLS_DIR=/home/mullr/src/sel4/seL4_tools
export SEL4_PLATFORM=pc99

cmake \
    -DKernelDebugBuild=ON \
    -DKernelPrinting=ON \
    -DKernelColourPrinting=ON \
    -DKernelUserStackTraceLength=16 \
    -DBuildWithCommonSimulationSettings=ON \
    -DKernelOptimisation=-O2 \
    -DKernelVerificationBuild=OFF \
    -DKernelBenchmarks=none \
    -DKernelFastpath=ON \
    -DLibSel4FunctionAttributes=public \
    -DKernelNumDomains=1 \
    -DHardwareDebugAPI=OFF \
    -DKernelFWholeProgram=OFF \
    -DKernelResetChunkBits=8 \
    -DLibSel4DebugAllocBufferEntries=0 \
    -DLibSel4DebugFunctionInstrumentation=none \
    -DKernelNumPriorities=256 \
    -DKernelStackBits=12 \
    -DKernelTimeSlice=5 \
    -DKernelTimerTickMS=2 \
    -DKernelSel4Arch=x86_64 \
    -DKernelPlatform=pc99 \
    -DKernelArch=x86 \
    -DKernelX86Sel4Arch=x86_64 \
    -DKernel64=ON \
    -DKernelX86MicroArch=nehalem \
    -DLibPlatSupportX86ConsoleDevice=com1 \
    -DKernelMaxNumNodes=1 \
    -DKernelRetypeFanOutLimit=16384 \
    -DKernelRootCNodeSizeBits=19 \
    -DKernelMaxNumBootinfoUntypedCaps=230 \
    -DKernelSupportPCID=OFF \
    -DKernelCacheLnSz=64 \
    -DKernelDebugDisablePrefetchers=OFF \
    -DKernelExportPMCUser=OFF \
    -DKernelFPU=FXSAVE \
    -DKernelFPUMaxRestoresSinceSwitch=64 \
    -DKernelFSGSBase=msr \
    -DKernelHugePage=ON \
    -DKernelIOMMU=OFF \
    -DKernelIRQController=IOAPIC \
    -DKernelIRQReporting=ON \
    -DKernelLAPICMode=XAPIC \
    -DKernelMaxNumIOAPIC=1 \
    -DKernelMaxNumWorkUnitsPerPreemptio=100 \
    -DKernelMultiboot1Header=ON \
    -DKernelMultiboot2Header=ON \
    -DKernelMultibootGFXMode=none \
    -DKernelSkimWindow=ON \
    -DKernelSyscall=syscall \
    -DKernelVTX=OFF \
    -DKernelX86DangerousMSR=OFF \
    -DKernelX86IBPBOnContextSwitch=OFF \
    -DKernelX86IBRSMode=ibrs_none \
    -DKernelX86RSBOnContextSwitch=OFF \
    -DKernelXSaveSize=576 \
    -DLinkPageSize=4096 \
    -DUserLinkerGCSections=OFF \
    -DCMAKE_TOOLCHAIN_FILE=${KERNEL}/gcc.cmake \
                          -DKERNEL_PATH=${KERNEL} \
                          -G Ninja \
                          ..

ninja
cd ..

qemu-system-x86_64  -cpu Nehalem,-vme,+pdpe1gb,-xsave,-xsaveopt,-xsavec,-fsgsbase,-invpcid,enforce -nographic -serial mon:stdio -m size=3G -kernel kernel-build/images/kernel-x86_64-pc99 -initrd kernel-build/images/root_task-image-x86_64-pc99
