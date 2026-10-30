# Android testing and analysis research

Research only, 30 October 2026. No emulator, device, APK, runtime or tool was acquired or executed. Recommendations below require later P4/P5 scope approval. Sources are moving upstream documentation, not pinned artifact qualification; see [source register](post-p3-sources.md).

## Device runtime comparison

| Candidate | Fit and architecture | Isolation, licensing and maintenance caveats | Proposed disposition |
| --- | --- | --- | --- |
| AOSP Android Emulator | App development/UI tests; matching x86_64 guest on x86_64 KVM avoids instruction translation. ARM guest/device coverage must be tested separately. | Virtual device boundary still needs host process/egress limits. Emulator code, SDK distribution and guest images have separate terms; AOSP image does not imply Google APIs/Play images are FOSS. | Primary app-testing candidate: explicitly selected AOSP image and verified emulator build, subject to artifact/license gate. |
| Cuttlefish | Configurable virtual Android on Linux x86 and ARM64; better candidate for platform/system fidelity and headless automation. | Larger image/build/setup burden; KVM/crosvm/device access needs separate authorization and qualification. AOSP mix of Apache/GPL/component notices. | Primary fallback for platform-level or headless fixtures when app emulator is unsuitable. |
| Waydroid | Container-based Android integrated with Wayland desktops. | Shares host kernel, binder/device and desktop integration widen exposure. GPL-3.0 project with separately licensed images. NVIDIA documentation describes software rendering fallback; no KDE/NVIDIA performance assumption. | Optional desktop convenience fallback for trusted app fixtures; not the default hostile-APK boundary. |
| redroid | Android containers, multi-architecture and GPU options; headless/server use. | Documented privileged launch is too broad as a default. Shared host kernel and binder risks; wrapper/image licenses must be inventoried independently. | Research alternative for controlled CI hosts; reject privileged default for Rivet projects. |
| QEMU + Android-x86 | General VM control, snapshots and x86 guests; useful compatibility experiments. | QEMU security depends on machine/device/backend configuration. Android-x86 homepage news is old; patch freshness and image provenance need investigation. Apache/GPL component mix, QEMU GPLv2 family. | Specialist legacy fallback only, not a maintained Android-version promise. |
| Physical Android device | Real ARM ABI, vendor GPU and device-specific behavior. | Separate USB/ADB grant, test-only device/account, data wipe and ownership checks; never assume personal-device authorization. | Fallback for architecture/GPU gaps after explicit approval. |

[Android acceleration](https://developer.android.com/studio/run/emulator-acceleration) establishes Linux KVM and CPU/guest matching constraints; [Cuttlefish](https://source.android.com/docs/devices/cuttlefish) documents local Linux architecture support. [Waydroid GPU fallback](https://docs.waydro.id/faq/get-waydroid-to-work-through-a-vm), [redroid](https://github.com/remote-android/redroid-doc), [QEMU security](https://www.qemu.org/docs/master/system/security.html) and [Android-x86](https://www.android-x86.org/) support the comparison. Selection and threat dispositions are Rivet recommendations, not upstream endorsements.

Fedora KDE Wayland/NVIDIA is a **future test target**, not verified support. Qualify exact kernel/KVM permissions, SELinux denials, binder support where required, compositor, driver and render backend. Test software rendering first as a supported fallback candidate; compare correctness, CPU load and frame latency against permitted GPU acceleration. No host package, kernel module, group membership or configuration changes are authorized here. On x86 hosts, ARM-only native APK libraries may require an ARM device/image or separately licensed translation; never silently add proprietary bridges. KVM accelerates compatible architectures; it does not make all guest ABIs interchangeable.

## Control and APK tools

| Tool | Purpose and upstream license snapshot | Fit / limit |
| --- | --- | --- |
| ADB | Device install/debug/shell, AOSP component terms | Rivet P4 transport candidate; pin serial/transport identity and restrict subcommands. ADB is powerful execution access, not a sandbox. |
| UI Automator | Android UI/system-app testing, AndroidX Apache-2.0 family | Primary semantic UI test candidate; wait/assert selectors, fixture APK and test instrumentation. Does not cover every custom-rendered surface. |
| scrcpy | Mirroring/input, Apache-2.0 | Optional human observation/control; capture/input separate grants. Not semantic test assertions or an emulator. |
| JADX | DEX/APK Java decompilation, Apache-2.0 | Unrivet P5 primary readable-code candidate. Output can be incomplete or wrong; retain original bytecode and warnings. |
| Apktool | Resource/manifest/smali decode/rebuild, Apache-2.0 | Complement JADX for resources and structure. Rebuild/signature changes are separate mutations, never equivalent to original APK. |
| Androguard | Programmatic APK/DEX analysis, Apache-2.0 snapshot | Structured metadata/cross-reference alternative; qualify version/API/dependencies before making it a shared service. |
| Frida | Runtime instrumentation, wxWindows Library Licence 3.1 family | Optional dynamic specialist. Server/root or gadget injection changes threat and license surface; require explicit target authority and version match. |
| MobSF | Integrated static/dynamic security workflow, GPL-3.0 family | Optional broad specialist, not default daemon/UI. Higher dependency/service footprint; prevent automatic external scanning/upload and validate findings. |

Primary links: [ADB](https://developer.android.com/tools/adb), [UI Automator](https://developer.android.com/training/testing/other-components/ui-automator), [scrcpy](https://github.com/Genymobile/scrcpy), [JADX](https://github.com/skylot/jadx), [Apktool](https://apktool.org/), [Androguard](https://github.com/androguard/androguard), [Frida](https://github.com/frida/frida), [MobSF](https://github.com/MobSF/Mobile-Security-Framework-MobSF). Licenses are project-level observations, not transitive/binary redistribution clearance. Preserve exact SPDX suffixes from a later pinned artifact rather than inferring them from GitHub labels.

Alternatives: AndroidX Espresso for in-app instrumented assertions, Appium for broader automation protocols, Gradle-managed virtual devices for build integration, and direct manifest/DEX parsers for minimal static extraction. Qualify these only if UI Automator/ADB or selected static tools leave a demonstrated gap; do not add several overlapping stacks by default. Their exact release/license/support fit remains unverified in this pass.

## Proposed acceptance and threat gates

P4: separately approve existing-device or virtual-device acquisition, storage, `/dev/kvm`/USB/device access and bounded guest networking. Use self-owned fixture APKs, no personal accounts. Bind target serial plus boot identity at every action; reject unplug/replug/serial reuse and wrong foreground targets. Test install/run/assert/screenshot/uninstall, selector timeout, denied input/capture, stale UI tree, guest crash, cancellation and descendant cleanup. Record OS/image/tool versions, hashes and backend. Emulator networking and ADB ports must not expose host services or other devices; never bind control unauthenticated to the LAN.

P5: only Rivet-side optional-mod contracts in this repository. Static parsing runs offline under file/output/CPU/memory limits. Fixtures include malformed DEX/resources, archive traversal, symlinks, decompression bombs, huge graphs, obfuscated code and native libraries. Preserve original APK hash and signatures; label extracted/decompiled outputs as derivatives with tool/version/provenance. Dynamic tests use disposable isolated guests, separate instrumentation/network grants, timeouts and reset verification. Analysis text is untrusted input, not agent authority.

Measure cold/warm boot, test completion, idle CPU/RSS, disk/image/cache size and peak RAM/VRAM on generic older and modern Linux hosts, x86_64 and ARM64 where supported. Report GPU/software rendering, workload, repeats and failures. Upstream performance claims are not Rivet measurements. Compare correctness and cleanup before speed. Pin releases after reviewing changelog/security activity and image patch levels; no latest-version or maintenance guarantee is made here.
