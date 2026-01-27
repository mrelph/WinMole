# **Systems Architecture and Performance Engineering: A Comprehensive Guide for Windows Optimization Utility Development**

## **1\. Executive Summary: The Architecture of Latency and Throughput**

The development of a performance optimization utility for the Windows operating system requires a fundamental paradigm shift from the manufacturer's default configuration philosophy. Microsoft designs the Windows NT kernel and user space to prioritize distinct metrics: hardware compatibility, power efficiency (battery life), stability across diverse configurations, and throughput (bulk data transfer). Conversely, high-performance computing scenarios—specifically competitive gaming, real-time digital signal processing, and low-latency workstations—demand a prioritization of **interrupt determinism**, **minimized DPC (Deferred Procedure Call) latency**, and **input responsiveness**.

For a developer tasked with engineering an optimization application, the objective is not merely "cleaning" the system but re-architecting the scheduling and resource allocation logic of the OS. This report serves as a foundational architectural document, categorizing optimization vectors by their subsystem impact. It analyzes the specific registry modifications, service configurations, and kernel parameters necessary to transition Windows from a general-purpose state to a high-performance runtime environment.

The analysis is structured hierarchically, beginning with the highest-impact interventions in the Service Control Manager and Kernel Scheduler, proceeding through the Network Stack and Memory Management subsystems, and concluding with Hardware Interrupt handling and Power Management. Furthermore, this report integrates implementation strategies for the application developer, detailing the programmatic methods required to modify protected system settings safely using C\# and PowerShell interfaces.

## **2\. Category I: The Service Control Subsystem and Telemetry Reduction**

The most significant latency overhead in a modern Windows environment stems from the continuous background execution of non-essential services, telemetry agents, and provisioned Universal Windows Platform (UWP) applications. These processes consume CPU cycles, interrupt time, and memory bandwidth, often triggering non-deterministic context switches that manifest as "stutter" or input lag in real-time applications.

### **2.1. Telemetry and Diagnostic Data Collection Architectures**

Windows 11 utilizes a sophisticated telemetry framework designed to collect usage metrics, crash dumps, and compatibility data. While valuable for OS development, the active collection and transmission of this data create erratic background network and disk I/O activity.

#### **2.1.1. The Diagnostics Tracking Service (DiagTrack)**

At the core of the telemetry infrastructure lies the DiagTrack service, formally identified as the **Connected User Experiences and Telemetry** service.1 This service is responsible for the event-driven aggregation and transmission of diagnostic data to Microsoft endpoints.

* **Operational Impact:** DiagTrack frequently wakes the CPU from idle states to bundle data packets, a process that involves both disk reads (to gather logs) and network transmission. In latency-sensitive environments, this background activity competes with the foreground application for I/O bandwidth and scheduler attention.3  
* **Engineering Implementation:** The optimization utility must interface with the Service Control Manager (SCM) to fundamentally disable this service. Simply stopping the service is insufficient, as the OS creates triggers to restart it. The target configuration is StartType \= Disabled.  
* **Dependency Management:** DiagTrack often operates in conjunction with dmwappushservice (WAP Push Message Routing Service), which facilitates the routing of telemetry data over specific protocols.1 Both services must be targeted simultaneously to prevent "orphaned" telemetry processes from attempting to reconnect.

#### **2.1.2. The Application Experience and Compatibility Appraisers**

A frequently overlooked component of the telemetry stack is the **Microsoft Compatibility Appraiser**. This is not a service but a high-privilege scheduled task located at \\Microsoft\\Windows\\Application Experience\\.5

* **Mechanism of Action:** This task scans the file system and running processes to evaluate compatibility with future Windows updates. It is a primary source of "unexplained" high disk usage (100% disk usage spikes) on systems with mechanical hard drives or slower SSDs.6  
* **Developer Strategy:** An optimization application cannot rely solely on service management; it must include a module for **Task Scheduler** manipulation. The application needs to enumerate tasks in the Microsoft\\Windows\\Application Experience namespace and explicitly disable Microsoft Compatibility Appraiser, ProgramDataUpdater, and StartupAppTask.7  
* **Persistence Mechanisms:** Windows Update often re-enables these tasks. A robust optimization app acts as a state enforcement agent, periodically verifying that these tasks remain disabled or removing the trigger conditions that allow them to respawn.9

### **2.2. Automated Debloating Logics and "Provisioned" Packages**

A critical distinction for the application developer is the difference between "installed" apps and "provisioned" packages. Standard uninstallation removes an app for the current user, but "provisioned" packages remain in the system image (.wim or system store) and reinstall themselves for every new user profile or during major feature updates.10

#### **2.2.1. The "Provisioned" Package Architecture**

Windows uses the AppX and MSIX frameworks for modern applications. When a user logs in, the **AppX Deployment Service** (AppxSvc) scans the provisioned package list and installs missing apps.

* **Performance Implication:** This process consumes significant storage and background CPU during maintenance windows. Furthermore, many of these apps (e.g., "Your Phone," "Tips," "Feedback Hub") include background tasks that run even when the app is closed.  
* **Optimization Logic:** The optimization utility must utilize the PowerShell Get-AppxProvisionedPackage and Remove-AppxProvisionedPackage cmdlets (or their API equivalents) to scrub the system image, not just the user profile.10  
* **Safe-to-Remove Lists:** Analysis of community-driven debloating frameworks like **Sophia Script** and **Chris Titus Tech's WinUtil** reveals a consensus on safe-to-remove packages. These JSON-based configurations provide a blueprint for the application developer, categorizing apps into "Safe," "Advanced," and "Do Not Touch" (e.g., the Microsoft Store itself).12

#### **2.2.2. JSON-Based Configuration Management**

Leading optimization scripts leverage external configuration files (typically JSON) to maintain lists of services and apps. This separates the application logic from the data, allowing for rapid updates when Microsoft changes service names or adds new bloatware.

* **Implementation Model:** The optimization app should parse a remote or local tweaks.json file that defines the state of services. For instance, WinUtil defines services like WSearch (Windows Search) and SysMain with desired states (Disabled or Manual).14  
* **State Enforcement:** Instead of hard-coding Disabled, the application should support a "Manual" state for services like MapsBroker or XboxGipSvc. Setting a service to Manual allows it to start if a user explicitly launches a dependent application, preventing system instability while maintaining zero overhead during normal use.15

### **2.3. Scheduled Task Pruning**

Beyond services, the Windows Task Scheduler is a repository for hundreds of maintenance jobs.

* **Target Identification:** The optimization app should target:  
  * \\Microsoft\\Windows\\Maps\\MapsToastTask & MapsUpdateTask: Unnecessary if offline maps are not used.12  
  * \\Microsoft\\Windows\\XblGameSave\\XblGameSaveTask: Can be disabled if the user does not use Xbox Live cloud saves, reducing periodic network checks.12  
  * \\Microsoft\\Windows\\Customer Experience Improvement Program\\Consolidator: A core telemetry uploader.7  
* **Programmatic Access:** Accessing these tasks programmatically requires Administrator privileges. The application must use the COM-based Task Scheduler interfaces or wrap PowerShell commands (Disable-ScheduledTask) to modify these states.17

| Service / Task Name | System Name | Action | Performance Impact |
| :---- | :---- | :---- | :---- |
| Connected User Experiences | DiagTrack | Disable | High (CPU/Network) |
| WAP Push Message Routing | dmwappushservice | Disable | Medium (Memory) |
| Compatibility Appraiser | Microsoft Compatibility Appraiser | Disable | High (Disk I/O) |
| Customer Experience Consolidator | Consolidator | Disable | Medium (Network) |
| Windows Search (Optional) | WSearch | Disable/Manual | High (Disk Indexing) |
| SysMain (SuperFetch) | SysMain | Disable (SSD) | High (CPU/Disk) |

## **3\. Category II: Kernel Scheduling and Processor Topology (High Impact)**

Once the system's background noise is suppressed, the optimization utility must turn its focus to the Kernel Scheduler. The default Windows scheduler is designed for "fairness"—ensuring that background tasks (like virus scans or file indexing) get adequate CPU time alongside foreground applications. For a performance-focused system, "fairness" is undesirable; the goal is **absolute priority** for the active workload.

### **3.1. Quantum Dynamics and Priority Separation**

The Win32PrioritySeparation registry value is the primary lever for adjusting how the scheduler allocates time slices (quantums) to threads.19

* **Mechanism:** This value is a bitmask that controls two variables:  
  1. **Quantum Length:** Whether threads run for short intervals (allowing faster context switching and responsiveness) or long intervals (maximizing throughput by reducing cache flushes).  
  2. **Foreground Boost:** How much extra quantum the foreground window receives compared to background processes.  
* **Registry Path:** HKLM\\SYSTEM\\CurrentControlSet\\Control\\PriorityControl\\Win32PrioritySeparation.  
* **Optimization Values:**  
  * **26 (Hex) / 38 (Decimal):** This configuration sets **Short Quantums** and **Variable Length** with a significantly higher boost for the foreground process. It is the aggressive standard for gaming, ensuring the game loop processes input as frequently as possible.19  
  * **28 (Hex) / 40 (Decimal):** This sets **Short Quantums** but with **Fixed Length** (no boost). Some audio engineers and competitive gamers prefer this for consistency, arguing that variable boosting can introduce jitter in frame times.20  
  * **16 (Hex) / 22 (Decimal):** **Long Quantums**. This minimizes context switching overhead, theoretically increasing raw FPS (frames per second) in CPU-bound scenarios, though potentially at the cost of input "snap".20  
* **Application Logic:** The optimization app should ideally offer a toggle or a slider, allowing the user to select between "Max Responsiveness" (Short Quantums) and "Max Throughput" (Long Quantums).

### **3.2. Multimedia Class Scheduler Service (MMCSS) Tuning**

Windows uses the Multimedia Class Scheduler Service (MMCSS) to ensure glitch-free audio and video playback. By default, MMCSS reserves a portion of CPU cycles for low-priority background threads to prevent system starvation—a safeguard known as SystemResponsiveness.

* **The Bottleneck:** The default registry value SystemResponsiveness is often set to 20 (meaning 20% of CPU resources are guarded for background tasks). In a dedicated gaming or rendering session, this 20% headroom is wasted potential.22  
* **Optimization:** The optimization utility should modify HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\SystemResponsiveness to 0 or 10\. This signals the scheduler to release that reserved capacity to the multimedia workload.22  
* **GPU Priority Promotion:** Within the MMCSS registry structure (...\\Tasks\\Games), there are values for GPU Priority and Priority. Increasing GPU Priority to 8 and Priority to 6 can further hint the scheduler to prioritize the graphics pipeline, although the tangible impact varies by GPU driver version.24

### **3.3. Hybrid Architecture Management: The Core Parking Problem**

With the advent of Intel's hybrid architecture (12th Gen Alder Lake and newer), managing "Performance" (P-cores) and "Efficiency" (E-cores) has become a critical optimization frontier.

* **Core Parking:** To save power, Windows "parks" idle cores (puts them in deep C-states). Unparking a core takes time (micro-seconds), which translates to a stall in the processing pipeline.26  
* **The Hybrid Risk:** Aggressive core parking can lead to a situation where a high-performance thread is initially scheduled on a slower E-core or suffers latency while waiting for a P-core to wake up.  
* **Optimization Strategy:** The utility must interact with the **Power Configuration API** to disable core parking, specifically for the "High Performance" or "Ultimate Performance" power plans.  
  * **Registry Implementation:** This involves unhiding the "Processor performance core parking min cores" setting via the Attributes value in HKLM\\SYSTEM\\CurrentControlSet\\Control\\Power\\PowerSettings\\...\\dec35c318583.28  
  * **Target State:** Setting the minimum parked cores to 100% (or 0 depending on the logic implementation in the GUI) effectively keeps all cores active and unparked.  
  * **Impact:** This has been verified to fix stuttering issues in games like *Elden Ring* on hybrid CPUs, as threads are immediately serviced by an active P-core.30

## **4\. Category III: Network Stack Engineering (Medium Impact)**

The default TCP/IP stack in Windows is tuned for the early 2000s internet: reliable file transfers over lossy connections. It uses algorithms designed to coalesce data packets to save bandwidth. In modern high-bandwidth fiber environments, these algorithms introduce unnecessary latency.

### **4.1. Nagle’s Algorithm and the Delayed ACK Timer**

Nagle's Algorithm is a congestion control mechanism that combines small outgoing packets into a single larger segment. While efficient for bandwidth, it is disastrous for real-time applications where every keystroke or position update is a tiny packet that must be sent immediately.

* **The Interaction:** Nagle’s algorithm waits for an acknowledgment (ACK) before sending the next packet. Windows, by default, delays sending ACKs (Delayed ACK) by up to 200ms, hoping to bundle the ACK with outgoing data. This interaction causes a "deadlock" of latency.32  
* **Registry Implementation:** The optimization app must iterate through the network interfaces in HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces\\{NIC-ID} and inject two DWORD values:  
  1. TCPNoDelay \= 1: Disables Nagle’s Algorithm.  
  2. TcpAckFrequency \= 1: Disables Delayed ACK, forcing an immediate acknowledgment of every packet.33  
* **App Logic:** The application needs logic to identify the *active* network interface (as there may be many virtual adapters) to apply these tweaks correctly.27

### **4.2. Network Throttling Index**

Windows implements a throttling mechanism for non-multimedia network traffic (NetworkThrottlingIndex) to prevent network processing from overwhelming the CPU.

* **Default Behavior:** The default value (10 or 15\) restricts the processing rate of network packets. On modern multi-core CPUs, this protection is obsolete and acts as an artificial bandwidth cap.  
* **Optimization:** The utility should set HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\NetworkThrottlingIndex to FFFFFFFF (Hex). This completely disables the throttling logic, allowing the network stack to process packets as fast as the hardware allows.25

## **5\. Category IV: Memory and Storage Subsystems (Medium-High Impact)**

Optimizing memory management involves balancing the speed of RAM with the capacity of the disk.

### **5.1. The Large System Cache Dilemma**

The LargeSystemCache registry setting (HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management) controls whether the system maintains a standard size file system cache (8MB) or expands it to utilize available physical memory.39

* **Analysis:** Enabling this (1) mimics Windows Server behavior, caching huge amounts of file data in RAM. While this speeds up file server operations, it can be detrimental to desktop performance. The expanded cache competes with applications for physical RAM, potentially forcing the application's code or data to be paged out to the disk.  
* **Recommendation:** For a gaming or workstation optimization app, the safest default is **0 (Disabled)**. However, the app could offer a "Workstation Mode" that enables it for users dealing with massive file transfers.41

### **5.2. SysMain (SuperFetch) on Solid State Drives**

SysMain (formerly SuperFetch) preloads frequently used applications into standby memory.

* **The Modern Context:** On mechanical hard drives, this was essential. On NVMe SSDs with read speeds exceeding 3000MB/s, the loading time difference is negligible.  
* **The Overhead:** SysMain performs memory compression and page combining. For systems with 32GB+ of RAM and fast SSDs, this CPU overhead is often unjustified. Disabling SysMain eliminates the background I/O associated with preloading and repopulating the cache.43

### **5.3. Virtual Memory: The Static Pagefile Strategy**

A common myth is that users with large amounts of RAM (32GB+) should disable the pagefile entirely.

* **Why Disabling is Risky:** Windows is architected to expect a pagefile. It uses it for crash dumps and to back "commit" memory allocations that may never be accessed. Disabling it forces all commit charges to be backed by physical RAM, which can lead to "Out of Memory" errors even when RAM is free.45  
* **The Optimized Approach:** The optimization app should not disable the pagefile but rather **lock** it. By setting a static initial and maximum size (e.g., 2048MB or 16GB), the app prevents the OS from dynamically resizing the file, which causes disk fragmentation and I/O stalls during resize operations.47

### **5.4. NTFS Metadata Overhead**

The NTFS file system maintains legacy metadata that degrades performance over time.

* **8.3 Name Creation:** Windows generates DOS-compatible "short names" (e.g., PROGRA\~1) for every file. This adds processing overhead to every file creation.  
* **Last Access Update:** Windows updates a file's metadata every time it is read. This turns every "read" operation into a "write" operation.  
* **Implementation:** The utility should execute fsutil behavior set disable8dot3 1 and fsutil behavior set disablelastaccess 1 to eliminate this write amplification.49

## **6\. Category V: Hardware Interrupt Architecture and Drivers (Medium Impact)**

As hardware speeds increase, the latency of the communication channel between the device and the CPU becomes the bottleneck.

### **6.1. Message Signaled Interrupts (MSI) Mode**

Legacy devices communicated with the CPU using Line-Based Interrupts (IRQ), which rely on physical pins and shared lines. This often forces the GPU to wait in a queue behind the USB controller or Audio card to speak to the CPU.

* **MSI-X:** Modern PCIe devices support Message Signaled Interrupts (MSI), which write directly to a specific memory address to trigger an interrupt, bypassing the shared line.  
* **The Issue:** To maintain compatibility, Windows often installs GPU drivers (especially NVIDIA) in legacy line-based mode.  
* **The Fix:** The optimization utility should scan the Device Parameters\\Interrupt Management\\MessageSignaledInterruptProperties registry key for the GPU and set MSISupported to 1\.  
* **Impact:** This dramatically reduces DPC (Deferred Procedure Call) latency, which is the primary cause of audio crackles and "robotic" glitches during high system load. It allows the GPU to have a dedicated, low-latency line to the CPU.51

### **6.2. High Precision Event Timer (HPET)**

The synchronization of time is critical for game engines.

* **The Controversy:** There is a debate between using the motherboard's High Precision Event Timer (HPET) versus the CPU's Invariant Time Stamp Counter (TSC).  
* **Modern Consensus:** Forcing the use of HPET via bcdedit /set useplatformclock true is detrimental on modern systems because reading the timer from the motherboard (across the bus) is significantly slower than reading the on-die CPU timer.  
* **Recommendation:** The optimization app should execute bcdedit /deletevalue useplatformclock and bcdedit /deletevalue disabledynamictick. This allows Windows 10/11 to use the most efficient timer available (usually the TSC), which has been shown to improve FPS in titles like *Counter-Strike 2* by reducing the overhead of time-check calls.55

### **6.3. Driver Composition: NVCleanstall Methodology**

The standard NVIDIA driver package has bloated to over 800MB, containing USB-C drivers, Telemetry services, Shield Wireless controller drivers, and the GeForce Experience backend.

* **Optimization:** A "Driver Slimming" module in the app (or guidance to use tools like NVCleanstall) should strip these components.  
* **Core Components:** For a pure gaming rig, only the **Core Driver**, **PhysX**, and **HD Audio** are required.  
* **Latency Benefit:** Removing the telemetry and "Container" services reduces the number of background threads and hooks injected into game processes.59

## **7\. Category VI: Power Management and UI Responsiveness (Foundation)**

### **7.1. unlocking the "Ultimate Performance" Plan**

Windows 10 and 11 contain a hidden power profile derived from Windows Server editions, labeled "Ultimate Performance."

* **Architecture:** Unlike the "High Performance" plan, "Ultimate Performance" is optimized to eliminate micro-latencies associated with power state transitions. It attempts to keep CPU voltages and frequencies locked, preventing the processor from downclocking during micro-seconds of idle time.  
* **Activation:** The utility can enable this by running the powercfg command: powercfg \-duplicatescheme e9a42b02-d5df-448d-aa00-03f14749eb61.62

### **7.2. User Interface Latency**

While not affecting raw computational throughput, UI latency defines the "feel" of the OS.

* **MenuShowDelay:** Windows waits 400ms before expanding a hover menu. Reducing this registry value (HKCU\\Control Panel\\Desktop\\MenuShowDelay) to 0 or 20 makes the UI feel instant.64  
* **WaitToKillServiceTimeout:** Reducing this value in HKLM\\SYSTEM\\CurrentControlSet\\Control minimizes the time Windows waits for hung services during shutdown, speeding up the reboot cycle.22

## **8\. Implementation Guide: Engineering the Optimization App**

Building this utility requires navigating Windows security models. The registry keys discussed (HKLM) are protected and require Administrator privileges.

### **8.1. Programmatic Registry Access (C\#)**

The.NET Registry class allows for modification, but the application must request the appropriate access rights.

* **Manifest Requirement:** The application manifest must specify \<requestedExecutionLevel level="requireAdministrator" uiAccess="false" /\> to trigger the UAC prompt upon launch.66  
* **Handling Permissions:** Some keys are owned by TrustedInstaller. The application may need to programmatically seize ownership using RegistrySecurity and SetAccessControl.  
  * *Code Logic:* The app should instantiate a RegistrySecurity object, add an AccessRule granting "FullControl" to the "Administrators" group, and then apply this rule to the target key before attempting to write values.67

### **8.2. Service Management Wrapper**

The System.ServiceProcess.ServiceController class is the standard interface for managing services.

* **Robustness:** The app should implement a generic function that accepts a service name and a desired state (e.g., SetServiceStatus("DiagTrack", ServiceStartMode.Disabled)).  
* **Error Handling:** It must catch exceptions where services are marked as "Unstoppable" (e.g., core Windows kernel services) and log these exceptions rather than crashing.18

## **9\. Summary Comparison of Tweak Impact**

| Category | Optimization Target | Technical Mechanism | Impact Type |
| :---- | :---- | :---- | :---- |
| **Services** | Disable DiagTrack / Telemetry | Stops background data harvesting & upload | Reduced CPU Spikes |
| **Services** | Disable SysMain | Stops RAM compression & pre-loading | Reduced Disk I/O |
| **Scheduler** | Win32PrioritySeparation \= 26 | Prioritizes foreground window quantum | Input Responsiveness |
| **Scheduler** | Core Unparking | Prevents C-state sleep on P-cores | Eliminates Stutter |
| **Network** | Disable Nagle (TCPNoDelay) | Forces immediate packet transmission | Lower Ping |
| **Network** | NetworkThrottlingIndex \= FFFFFFFF | Removes throughput artificial cap | Network Throughput |
| **Interrupts** | MSI Mode (GPU) | Bypasses shared IRQ lines | Lower DPC Latency |
| **Storage** | Static Pagefile | Prevents dynamic resize fragmentation | System Stability |
| **Power** | Ultimate Performance Plan | Locks CPU frequency states | Consistent Frame Times |

## **10\. Conclusion**

The construction of a Windows optimization application is a sophisticated exercise in systems engineering. It requires peeling back the layers of abstraction Microsoft has placed over the NT kernel. By systematically addressing the subsystems outlined in this report—starting with the removal of telemetry overhead, tuning the scheduler for responsiveness, and optimizing the hardware interrupt path—developers can build a tool that delivers measurable improvements in latency and consistency.

The data indicates that while no single tweak transforms a system, the aggregation of these optimizations creates a "runtime environment" that is fundamentally distinct from a stock installation. It transforms Windows from a general-purpose OS designed for battery life and compatibility into a focused platform for high-performance computing.

### ---

**Citations**

1

#### **Works cited**

1. How to Disable Telemetry in Windows 11 | NinjaOne, accessed on January 27, 2026, [https://www.ninjaone.com/blog/how-to-disable-telemetry-in-windows-11/](https://www.ninjaone.com/blog/how-to-disable-telemetry-in-windows-11/)  
2. Guidance on disabling system services on Windows IoT Enterprise \- Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/windows/iot/iot-enterprise/optimize/services](https://learn.microsoft.com/en-us/windows/iot/iot-enterprise/optimize/services)  
3. Windows 11 Privacy Settings: Complete Setup Guide \- Aardwolf Security, accessed on January 27, 2026, [https://aardwolfsecurity.com/how-to-set-up-windows-11-for-maximum-privacy/](https://aardwolfsecurity.com/how-to-set-up-windows-11-for-maximum-privacy/)  
4. Security guidelines for system services in Windows Server 2016 \- Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/windows-server/security/windows-services/security-guidelines-for-disabling-system-services-in-windows-server](https://learn.microsoft.com/en-us/windows-server/security/windows-services/security-guidelines-for-disabling-system-services-in-windows-server)  
5. Persistence via TelemetryController Scheduled Task Hijack | Detection.FYI, accessed on January 27, 2026, [https://detection.fyi/elastic/detection-rules/windows/persistence\_via\_telemetrycontroller\_scheduledtask\_hijack/](https://detection.fyi/elastic/detection-rules/windows/persistence_via_telemetrycontroller_scheduledtask_hijack/)  
6. How to completely and permanently disable/delete Windows Compatibility Telemetry?, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/answers/questions/3987504/how-to-completely-and-permanently-disable-delete-w](https://learn.microsoft.com/en-us/answers/questions/3987504/how-to-completely-and-permanently-disable-delete-w)  
7. Task Scheduler tasks that can be disabled? \- Windows 11 Forum, accessed on January 27, 2026, [https://www.elevenforum.com/t/task-scheduler-tasks-that-can-be-disabled.23555/](https://www.elevenforum.com/t/task-scheduler-tasks-that-can-be-disabled.23555/)  
8. List of useless, safe to disable microsoft service : r/computers \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/computers/comments/ndj44p/list\_of\_useless\_safe\_to\_disable\_microsoft\_service/](https://www.reddit.com/r/computers/comments/ndj44p/list_of_useless_safe_to_disable_microsoft_service/)  
9. Is it okay? Microsoft compatibility telemetry using 96% of my cpu : r/computers \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/computers/comments/1bbzqmd/is\_it\_okay\_microsoft\_compatibility\_telemetry/](https://www.reddit.com/r/computers/comments/1bbzqmd/is_it_okay_microsoft_compatibility_telemetry/)  
10. How to debloat Windows 11 \- Atera, accessed on January 27, 2026, [https://www.atera.com/blog/how-to-debloat-windows-11/](https://www.atera.com/blog/how-to-debloat-windows-11/)  
11. farag2/Sophia-Script-for-Windows: :zap: The most powerful open source tweaker on GitHub for fine-tuning Windows 10 & Windows 11, accessed on January 27, 2026, [https://github.com/farag2/Sophia-Script-for-Windows](https://github.com/farag2/Sophia-Script-for-Windows)  
12. raw.githubusercontent.com, accessed on January 27, 2026, [https://raw.githubusercontent.com/farag2/Sophia-Script-for-Windows/master/src/Sophia\_Script\_for\_Windows\_11/Module/Sophia.psm1](https://raw.githubusercontent.com/farag2/Sophia-Script-for-Windows/master/src/Sophia_Script_for_Windows_11/Module/Sophia.psm1)  
13. ChrisTitusTech/winutil 24.08.30 on GitHub \- NewReleases.io, accessed on January 27, 2026, [https://newreleases.io/project/github/ChrisTitusTech/winutil/release/24.08.30](https://newreleases.io/project/github/ChrisTitusTech/winutil/release/24.08.30)  
14. winutil/config/tweaks.json at main · ChrisTitusTech/winutil · GitHub, accessed on January 27, 2026, [https://github.com/ChrisTitusTech/winutil/blob/main/config/tweaks.json](https://github.com/ChrisTitusTech/winutil/blob/main/config/tweaks.json)  
15. Set Services to Manual \- Winutil Documentation, accessed on January 27, 2026, [https://winutil.christitus.com/dev/tweaks/essential-tweaks/services/](https://winutil.christitus.com/dev/tweaks/essential-tweaks/services/)  
16. Disable services · Issue \#1036 · ChrisTitusTech/winutil \- GitHub, accessed on January 27, 2026, [https://github.com/ChrisTitusTech/winutil/issues/1036](https://github.com/ChrisTitusTech/winutil/issues/1036)  
17. Disable-ScheduledTask \- Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/powershell/module/scheduledtasks/disable-scheduledtask?view=windowsserver2025-ps](https://learn.microsoft.com/en-us/powershell/module/scheduledtasks/disable-scheduledtask?view=windowsserver2025-ps)  
18. How can I disable a scheduled task using Powershell? \- Server Fault, accessed on January 27, 2026, [https://serverfault.com/questions/601933/how-can-i-disable-a-scheduled-task-using-powershell](https://serverfault.com/questions/601933/how-can-i-disable-a-scheduled-task-using-powershell)  
19. Improve win32 priority seperation \- Windows 11 Forum, accessed on January 27, 2026, [https://www.elevenforum.com/t/improve-win32-priority-seperation.7631/](https://www.elevenforum.com/t/improve-win32-priority-seperation.7631/)  
20. The Benefits Of Priority Separation Control In Apex Legends \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/apexlegends/comments/1354pbl/the\_benefits\_of\_priority\_separation\_control\_in/](https://www.reddit.com/r/apexlegends/comments/1354pbl/the_benefits_of_priority_separation_control_in/)  
21. Windows Clean Install & Win32PrioritySeparation Question \!\!\! : r/pcmasterrace \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/pcmasterrace/comments/1nzgdfl/windows\_clean\_install\_win32priorityseparation/](https://www.reddit.com/r/pcmasterrace/comments/1nzgdfl/windows_clean_install_win32priorityseparation/)  
22. 5 registry tweaks I made to instantly speed up my Windows PC \- XDA Developers, accessed on January 27, 2026, [https://www.xda-developers.com/registry-tweaks-instantly-speed-up-windows-pc/](https://www.xda-developers.com/registry-tweaks-instantly-speed-up-windows-pc/)  
23. Multimedia Class Scheduler Service \- Win32 apps | Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/windows/win32/procthread/multimedia-class-scheduler-service](https://learn.microsoft.com/en-us/windows/win32/procthread/multimedia-class-scheduler-service)  
24. Smooth as butter gameplay after the following registry changes :: Returnal™ General Discussions \- Steam Community, accessed on January 27, 2026, [https://steamcommunity.com/app/1649240/discussions/0/3825298731244525539/](https://steamcommunity.com/app/1649240/discussions/0/3825298731244525539/)  
25. windows 11 tweaks guide. (Nagel's Algorithm and system gaming respons) : r/pcmasterrace, accessed on January 27, 2026, [https://www.reddit.com/r/pcmasterrace/comments/1enz1pf/windows\_11\_tweaks\_guide\_nagels\_algorithm\_and/](https://www.reddit.com/r/pcmasterrace/comments/1enz1pf/windows_11_tweaks_guide_nagels_algorithm_and/)  
26. Reduce DPC Latency & System Lag with Advanced Power Tweaks. \- YouTube, accessed on January 27, 2026, [https://www.youtube.com/watch?v=jMvktTwb5Co](https://www.youtube.com/watch?v=jMvktTwb5Co)  
27. \[Guide\] How to get the highest FPS possible\! \- Hypixel Forums, accessed on January 27, 2026, [https://hypixel.net/threads/guide-how-to-get-the-highest-fps-possible.1252264/](https://hypixel.net/threads/guide-how-to-get-the-highest-fps-possible.1252264/)  
28. How to disable CPU core parking in Windows 11 \- Danny Moran, accessed on January 27, 2026, [https://www.dannymoran.com/windows-cpu-core-parking/](https://www.dannymoran.com/windows-cpu-core-parking/)  
29. How to disable CPU parking in Windows 11? \- Super User, accessed on January 27, 2026, [https://superuser.com/questions/1879051/how-to-disable-cpu-parking-in-windows-11](https://superuser.com/questions/1879051/how-to-disable-cpu-parking-in-windows-11)  
30. PSA: If your Elden Ring is stuttering on PC, try disabling Core Parking : r/Eldenring \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/Eldenring/comments/1dhkgua/psa\_if\_your\_elden\_ring\_is\_stuttering\_on\_pc\_try/](https://www.reddit.com/r/Eldenring/comments/1dhkgua/psa_if_your_elden_ring_is_stuttering_on_pc_try/)  
31. How to disable CPU core parking in Windows 11 \- YouTube, accessed on January 27, 2026, [https://www.youtube.com/watch?v=sFz8HRk4WfI](https://www.youtube.com/watch?v=sFz8HRk4WfI)  
32. An excellent guide to optimizing your Windows 10 PC for gaming. : r/killerinstinct \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/killerinstinct/comments/4fcdhy/an\_excellent\_guide\_to\_optimizing\_your\_windows\_10/](https://www.reddit.com/r/killerinstinct/comments/4fcdhy/an_excellent_guide_to_optimizing_your_windows_10/)  
33. Legit way to improve your latency and avoid plumes (PC Only) : r/ffxiv \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/ffxiv/comments/1rq59z/legit\_way\_to\_improve\_your\_latency\_and\_avoid/](https://www.reddit.com/r/ffxiv/comments/1rq59z/legit_way_to_improve_your_latency_and_avoid/)  
34. SG :: Gaming Tweaks \- SpeedGuide, accessed on January 27, 2026, [https://www.speedguide.net/articles/gaming-tweaks-5812](https://www.speedguide.net/articles/gaming-tweaks-5812)  
35. Steamin yhteisö :: Opas :: Ultimate CS2 Ping Optimization Guide: Registry Tweaks and In-Game Settings \- Steam Community, accessed on January 27, 2026, [https://steamcommunity.com/sharedfiles/filedetails/?l=finnish\&id=3092452302](https://steamcommunity.com/sharedfiles/filedetails/?l=finnish&id=3092452302)  
36. Optimizing Your Windows Registry for Gaming Performance Using PowerShell \- Medium, accessed on January 27, 2026, [https://medium.com/aardvark-infinity/optimizing-your-windows-registry-for-gaming-performance-using-powershell-6fcf0f51f80f](https://medium.com/aardvark-infinity/optimizing-your-windows-registry-for-gaming-performance-using-powershell-6fcf0f51f80f)  
37. Guia :: Ultimate CS2 Ping Optimization Guide: Registry Tweaks and In-Game Settings, accessed on January 27, 2026, [https://steamcommunity.com/sharedfiles/filedetails/?l=brazilian\&id=3092452302](https://steamcommunity.com/sharedfiles/filedetails/?l=brazilian&id=3092452302)  
38. Ping Problems \- Microsoft Q\&A, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/answers/questions/2420607/ping-problems](https://learn.microsoft.com/en-us/answers/questions/2420607/ping-problems)  
39. Windows 11 Fix and Tweaks – All-in-One Batch Script \- GitHub, accessed on January 27, 2026, [https://github.com/kubaam/Windows-11-Fix-Tweaks](https://github.com/kubaam/Windows-11-Fix-Tweaks)  
40. Fix Ram Micro Stutters – Optimize File Cache And Standby Memory for Smoother FPS, accessed on January 27, 2026, [https://www.youtube.com/watch?v=\_dFDgcOuvLY](https://www.youtube.com/watch?v=_dFDgcOuvLY)  
41. Memory Limits for Windows and Windows Server Releases \- Win32 apps | Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/windows/win32/memory/memory-limits-for-windows-releases](https://learn.microsoft.com/en-us/windows/win32/memory/memory-limits-for-windows-releases)  
42. tweaking hardware komputer untuk meningkatkan kecepatan dan stabilitas penggunaan software komputer \- Sriwijaya University Repository, accessed on January 27, 2026, [https://repository.unsri.ac.id/171912/25/RAMA\_56401\_09030582125015.pdf](https://repository.unsri.ac.id/171912/25/RAMA_56401_09030582125015.pdf)  
43. Changing this option could IMPROVE FPS and reduce STUTTERS in MOST GAMES\!, accessed on January 27, 2026, [https://www.youtube.com/watch?v=E\_k8PaPMng4](https://www.youtube.com/watch?v=E_k8PaPMng4)  
44. Removes some bloatware, cleans up registry keys, and makes Windows run better overall., accessed on January 27, 2026, [https://gist.github.com/itsnebulalol/e54614e5055bfe9aa820d807bfca62d3](https://gist.github.com/itsnebulalol/e54614e5055bfe9aa820d807bfca62d3)  
45. Page file in 10/11, but 32gb of RAM : r/windows \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/windows/comments/1cx2jtz/page\_file\_in\_1011\_but\_32gb\_of\_ram/](https://www.reddit.com/r/windows/comments/1cx2jtz/page_file_in_1011_but_32gb_of_ram/)  
46. Paging File in Windows 11 \- System managed or Custom Sized, accessed on January 27, 2026, [https://www.elevenforum.com/t/paging-file-in-windows-11-system-managed-or-custom-sized.453/](https://www.elevenforum.com/t/paging-file-in-windows-11-system-managed-or-custom-sized.453/)  
47. How to determine the appropriate page file size for 64-bit versions of Windows \- Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/how-to-determine-the-appropriate-page-file-size-for-64-bit-versions-of-windows](https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/how-to-determine-the-appropriate-page-file-size-for-64-bit-versions-of-windows)  
48. Pagefile ON vs OFF | Which performs better? \#gaming \#pagefile \#performance \- YouTube, accessed on January 27, 2026, [https://www.youtube.com/watch?v=bUJZ099KhyM](https://www.youtube.com/watch?v=bUJZ099KhyM)  
49. fsutil behavior \- Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/fsutil-behavior](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/fsutil-behavior)  
50. Configuring NTFS file system for performance \- Server Fault, accessed on January 27, 2026, [https://serverfault.com/questions/46881/configuring-ntfs-file-system-for-performance](https://serverfault.com/questions/46881/configuring-ntfs-file-system-for-performance)  
51. Introduction to Message-Signaled Interrupts \- Windows drivers | Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/introduction-to-message-signaled-interrupts](https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/introduction-to-message-signaled-interrupts)  
52. MSI mode on GPU's : r/OptimizedGaming \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/OptimizedGaming/comments/107blhi/msi\_mode\_on\_gpus/](https://www.reddit.com/r/OptimizedGaming/comments/107blhi/msi_mode_on_gpus/)  
53. MSI Mode \- which Interrupt Policy and Priority to choose? | TechPowerUp Forums, accessed on January 27, 2026, [https://www.techpowerup.com/forums/threads/msi-mode-which-interrupt-policy-and-priority-to-choose.305797/](https://www.techpowerup.com/forums/threads/msi-mode-which-interrupt-policy-and-priority-to-choose.305797/)  
54. Message Signal Interrupts \- Yes or No? | TechPowerUp Forums, accessed on January 27, 2026, [https://www.techpowerup.com/forums/threads/message-signal-interrupts-yes-or-no.284717/](https://www.techpowerup.com/forums/threads/message-signal-interrupts-yes-or-no.284717/)  
55. Avoid blindly applying "pro" settings: how I gained \+141 FPS by disabling useplatformclock tweaks on CS2 : r/counterstrike2 \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/counterstrike2/comments/1lfp71r/avoid\_blindly\_applying\_pro\_settings\_how\_i\_gained/](https://www.reddit.com/r/counterstrike2/comments/1lfp71r/avoid_blindly_applying_pro_settings_how_i_gained/)  
56. revisiting hpet bcdedit tweaks: what are your timer bench results and settings?, accessed on January 27, 2026, [https://www.techpowerup.com/forums/threads/revisiting-hpet-bcdedit-tweaks-what-are-your-timer-bench-results-and-settings.326187/](https://www.techpowerup.com/forums/threads/revisiting-hpet-bcdedit-tweaks-what-are-your-timer-bench-results-and-settings.326187/)  
57. PSA: Disable HPET (High Precision Event Timer) for smoother gameplay with more FPS. : r/PUBATTLEGROUNDS \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/PUBATTLEGROUNDS/comments/afyx1f/psa\_disable\_hpet\_high\_precision\_event\_timer\_for/](https://www.reddit.com/r/PUBATTLEGROUNDS/comments/afyx1f/psa_disable_hpet_high_precision_event_timer_for/)  
58. AMD: HPET On or Off? | Off \= Less latency & stuttering, More FPS in MOST GAMES\!, accessed on January 27, 2026, [https://hub.tcno.co/videos/amd-hpet-on-or-off-off-less-latency-stuttering-more-fps-in-most-games/](https://hub.tcno.co/videos/amd-hpet-on-or-off-off-less-latency-stuttering-more-fps-in-most-games/)  
59. Optimizing Your NVCleanstall Settings for Peak Performance \- Oreate AI Blog, accessed on January 27, 2026, [https://www.oreateai.com/blog/optimizing-your-nvcleanstall-settings-for-peak-performance/5d9cc77f5bd653431c3903219196d11a](https://www.oreateai.com/blog/optimizing-your-nvcleanstall-settings-for-peak-performance/5d9cc77f5bd653431c3903219196d11a)  
60. NVCleanstall | NVIDIA GeForce Forums, accessed on January 27, 2026, [https://www.nvidia.com/en-us/geforce/forums/3d-vision/41/299061/nvcleanstall/](https://www.nvidia.com/en-us/geforce/forums/3d-vision/41/299061/nvcleanstall/)  
61. Install a CLEAN NVIDIA Driver using NVCleanstall (No Bloat, No Telemetry\!) \- YouTube, accessed on January 27, 2026, [https://www.youtube.com/watch?v=VEWzJyTUpno](https://www.youtube.com/watch?v=VEWzJyTUpno)  
62. How to enable Ultimate Performance Power Plan in Windows 10? \- Super User, accessed on January 27, 2026, [https://superuser.com/questions/1327298/how-to-enable-ultimate-performance-power-plan-in-windows-10](https://superuser.com/questions/1327298/how-to-enable-ultimate-performance-power-plan-in-windows-10)  
63. How to Enable the Ultimate Performance Power Plan in Windows 10 \- How-To Geek, accessed on January 27, 2026, [https://www.howtogeek.com/368781/how-to-enable-ultimate-performance-power-plan-in-windows-10/](https://www.howtogeek.com/368781/how-to-enable-ultimate-performance-power-plan-in-windows-10/)  
64. Make Windows 11 less annoying with these 11 Registry tweaks \- The Register, accessed on January 27, 2026, [https://www.theregister.com/2025/09/21/windows\_11\_registry\_hacks\_regedit/](https://www.theregister.com/2025/09/21/windows_11_registry_hacks_regedit/)  
65. Make Your Windows Faster \- Daniel Andrade, accessed on January 27, 2026, [https://danielandrade.net/posts/make-windows-faster-optimization-tips/](https://danielandrade.net/posts/make-windows-faster-optimization-tips/)  
66. How to handle registry modifications that require admin privileges? \- Stack Overflow, accessed on January 27, 2026, [https://stackoverflow.com/questions/29312292/how-to-handle-registry-modifications-that-require-admin-privileges](https://stackoverflow.com/questions/29312292/how-to-handle-registry-modifications-that-require-admin-privileges)  
67. RegistryKey.SetAccessControl(RegistrySecurity) Method (Microsoft.Win32), accessed on January 27, 2026, [https://learn.microsoft.com/en-us/dotnet/api/microsoft.win32.registrykey.setaccesscontrol?view=net-10.0](https://learn.microsoft.com/en-us/dotnet/api/microsoft.win32.registrykey.setaccesscontrol?view=net-10.0)  
68. RegistrySecurity Access is denied. C\# \- Stack Overflow, accessed on January 27, 2026, [https://stackoverflow.com/questions/6455691/registrysecurity-access-is-denied-c-sharp](https://stackoverflow.com/questions/6455691/registrysecurity-access-is-denied-c-sharp)  
69. How do I programmatically give ownership of a Registry Key to Administrators?, accessed on January 27, 2026, [https://stackoverflow.com/questions/38448390/how-do-i-programmatically-give-ownership-of-a-registry-key-to-administrators](https://stackoverflow.com/questions/38448390/how-do-i-programmatically-give-ownership-of-a-registry-key-to-administrators)  
70. Windows registry information for advanced users \- Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/troubleshoot/windows-server/performance/windows-registry-advanced-users](https://learn.microsoft.com/en-us/troubleshoot/windows-server/performance/windows-registry-advanced-users)  
71. Set Display for Performance \- Winutil Documentation, accessed on January 27, 2026, [https://winutil.christitus.com/dev/tweaks/z--advanced-tweaks---caution/display/](https://winutil.christitus.com/dev/tweaks/z--advanced-tweaks---caution/display/)  
72. Performance Tuning \- NVIDIA Docs, accessed on January 27, 2026, [https://docs.nvidia.com/networking/display/winof2v237/performance+tuning](https://docs.nvidia.com/networking/display/winof2v237/performance+tuning)  
73. Is it really necessary to debloat windows 11? And why do Youtubers always say that you get the best performance or better privacy by debloating windows 11? : r/Windows11 \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/Windows11/comments/16ii00j/is\_it\_really\_necessary\_to\_debloat\_windows\_11\_and/](https://www.reddit.com/r/Windows11/comments/16ii00j/is_it_really_necessary_to_debloat_windows_11_and/)  
74. Breaking down total system latency and explaining some tweaks (big post) \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/OptimizedGaming/comments/1az9i4m/breaking\_down\_total\_system\_latency\_and\_explaining/](https://www.reddit.com/r/OptimizedGaming/comments/1az9i4m/breaking_down_total_system_latency_and_explaining/)  
75. TCP optimization for network performance and resiliency | Compute Engine | Google Cloud Documentation, accessed on January 27, 2026, [https://docs.cloud.google.com/compute/docs/networking/tcp-optimization-for-network-performance-in-gcp-and-hybrid](https://docs.cloud.google.com/compute/docs/networking/tcp-optimization-for-network-performance-in-gcp-and-hybrid)  
76. Troubleshooting guide on TCP/IP performance issues \- Windows Server \- Microsoft Learn, accessed on January 27, 2026, [https://learn.microsoft.com/en-us/troubleshoot/windows-server/networking/overview-of-tcpip-performance](https://learn.microsoft.com/en-us/troubleshoot/windows-server/networking/overview-of-tcpip-performance)  
77. SG :: TCP Optimizer 4 Documentation \- Windows 7, 8, 10, 2012-2019 Server \- SpeedGuide, accessed on January 27, 2026, [https://www.speedguide.net/articles/tcp-optimizer-4-documentation-windows-7-8-10-2012-5821](https://www.speedguide.net/articles/tcp-optimizer-4-documentation-windows-7-8-10-2012-5821)  
78. Use properly " Message Signaled-Based Interrupts " (MSI Tool) \- YouTube, accessed on January 27, 2026, [https://www.youtube.com/watch?v=DCbeey5v\_T4](https://www.youtube.com/watch?v=DCbeey5v_T4)  
79. How to optimize and squeeze a few more FPS for Starfield by enabling MSI mode in your GPU \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/Starfield/comments/1665ds8/how\_to\_optimize\_and\_squeeze\_a\_few\_more\_fps\_for/](https://www.reddit.com/r/Starfield/comments/1665ds8/how_to_optimize_and_squeeze_a_few_more_fps_for/)  
80. Message Signal Interrupts \- RTX 30xx Performance Boost?, accessed on January 27, 2026, [https://forums.flightsimulator.com/t/message-signal-interrupts-rtx-30xx-performance-boost/532185](https://forums.flightsimulator.com/t/message-signal-interrupts-rtx-30xx-performance-boost/532185)  
81. Sophia Script for Windows download | SourceForge.net, accessed on January 27, 2026, [https://sourceforge.net/projects/sophia-script-windows.mirror/](https://sourceforge.net/projects/sophia-script-windows.mirror/)  
82. I used Sophia Script to take back control of Windows 11, and I wish I did it sooner, accessed on January 27, 2026, [https://www.xda-developers.com/sophia-script-returns-control-windows-11/](https://www.xda-developers.com/sophia-script-returns-control-windows-11/)  
83. Sophia Script for Windows \- Customize Windows 11 and 10, accessed on January 27, 2026, [https://www.elevenforum.com/t/sophia-script-for-windows-customize-windows-11-and-10.870/](https://www.elevenforum.com/t/sophia-script-for-windows-customize-windows-11-and-10.870/)  
84. Just installed windows 10\. What are all the thing i need to do to remove bloatware and optimize it for gaming and work? : r/pcmasterrace \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/pcmasterrace/comments/866r5r/just\_installed\_windows\_10\_what\_are\_all\_the\_thing/](https://www.reddit.com/r/pcmasterrace/comments/866r5r/just_installed_windows_10_what_are_all_the_thing/)  
85. Windows Services You Can Safely Disable to Improve Performance \- WindowsTechies, accessed on January 27, 2026, [https://windowstechies.com/guides/disable-services/](https://windowstechies.com/guides/disable-services/)  
86. MAXIMIZE PC PERFORMANCE | Win32 Priority Separation \- YouTube, accessed on January 27, 2026, [https://www.youtube.com/watch?v=uCUimS17yJ8](https://www.youtube.com/watch?v=uCUimS17yJ8)  
87. What are your favorite windows registry tweaks/hacks? Please share... \- TechRepublic, accessed on January 27, 2026, [https://www.techrepublic.com/forums/discussions/what-are-your-favorite-windows-registry-tweaks-hacks-please-share/](https://www.techrepublic.com/forums/discussions/what-are-your-favorite-windows-registry-tweaks-hacks-please-share/)  
88. Additional Windows 11 and BIOS gaming-related optimizations \- Kartones' Blog, accessed on January 27, 2026, [https://blog.kartones.net/post/additional-windows-11-and-bios-gaming-related-optimizations/](https://blog.kartones.net/post/additional-windows-11-and-bios-gaming-related-optimizations/)  
89. 35\. What are the best TCP Optimizer settings for gaming ? :: SG FAQ \- SpeedGuide, accessed on January 27, 2026, [https://www.speedguide.net/faq/35.-what-are-the-best-tcp-optimizer-settings-for-gaming-474](https://www.speedguide.net/faq/35.-what-are-the-best-tcp-optimizer-settings-for-gaming-474)  
90. Timer Tweaks Benchmarked \- TweakHound, accessed on January 27, 2026, [https://www.tweakhound.com/2014/01/30/timer-tweaks-benchmarked/](https://www.tweakhound.com/2014/01/30/timer-tweaks-benchmarked/)  
91. Cracking sounds \- Page 3 \- Republic of Gamers Forum \- 869985, accessed on January 27, 2026, [https://rog-forum.asus.com/t5/z370-z390/cracking-sounds/td-p/869985/page/3](https://rog-forum.asus.com/t5/z370-z390/cracking-sounds/td-p/869985/page/3)  
92. How to enable ULTIMATE PERFORMANCE Plan in Windows \- YouTube, accessed on January 27, 2026, [https://www.youtube.com/watch?v=XU9bfsF7KpA](https://www.youtube.com/watch?v=XU9bfsF7KpA)  
93. Hidden Windows 10 Ultimate Power Plan : r/EASPORTSWRC \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/EASPORTSWRC/comments/1cpp0qt/hidden\_windows\_10\_ultimate\_power\_plan/](https://www.reddit.com/r/EASPORTSWRC/comments/1cpp0qt/hidden_windows_10_ultimate_power_plan/)  
94. PSA: Nvidia Platform Controllers and Framework & NVCleanstall : r/eluktronics \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/eluktronics/comments/13vsskr/psa\_nvidia\_platform\_controllers\_and\_framework/](https://www.reddit.com/r/eluktronics/comments/13vsskr/psa_nvidia_platform_controllers_and_framework/)  
95. Windows Registry Guide: Proven Steps for Safe Editing and, accessed on January 27, 2026, [https://caasify.com/windows-registry-guide-safe-editing-optimization/](https://caasify.com/windows-registry-guide-safe-editing-optimization/)  
96. My settings to fix CS2 performance issues and have a smooth gameplay. (Nvidia Cards ONLY) \- Reddit, accessed on January 27, 2026, [https://www.reddit.com/r/cs2/comments/1atrsp4/my\_settings\_to\_fix\_cs2\_performance\_issues\_and/](https://www.reddit.com/r/cs2/comments/1atrsp4/my_settings_to_fix_cs2_performance_issues_and/)  
97. Encrypt Your Windows Pagefile To Improve Security \- gHacks Tech News, accessed on January 27, 2026, [https://www.ghacks.net/2011/04/04/encrypt-your-windows-pagefile-to-improve-security/](https://www.ghacks.net/2011/04/04/encrypt-your-windows-pagefile-to-improve-security/)  
98. VeraCrypt / Forums / Technical Topics: Data Leaks \-\> Paging File \- SourceForge, accessed on January 27, 2026, [https://sourceforge.net/p/veracrypt/discussion/technical/thread/edf2c22f/](https://sourceforge.net/p/veracrypt/discussion/technical/thread/edf2c22f/)  
99. OBS 25.x.x support for NVENC with "stripped" Nvidia drivers possible?, accessed on January 27, 2026, [https://obsproject.com/forum/threads/obs-25-x-x-support-for-nvenc-with-stripped-nvidia-drivers-possible.125948/](https://obsproject.com/forum/threads/obs-25-x-x-support-for-nvenc-with-stripped-nvidia-drivers-possible.125948/)  
100. Safety of disabling services in Windows 10 and 11 \- GitHub Gist, accessed on January 27, 2026, [https://gist.github.com/Aldaviva/0eb62993639da319dc456cc01efa3fe5](https://gist.github.com/Aldaviva/0eb62993639da319dc456cc01efa3fe5)  
101. List of 20 Windows Services Safe To Disable For Better Performance | PDF \- Scribd, accessed on January 27, 2026, [https://www.scribd.com/document/894293281/List-of-20-Windows-Services-Safe-to-Disable-for-Better-Performance](https://www.scribd.com/document/894293281/List-of-20-Windows-Services-Safe-to-Disable-for-Better-Performance)