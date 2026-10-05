# 原生 worker 资源限制

2026-10-06 · `mo-native-limits` 已实现并在 macOS arm64 实际验证；视频 worker 已接入启动限制；这不代表完整产品或系统调用隔离验收。

## 语义

这是仅供 disposable worker 启动时调用的系统层，不进入纯文档/视频领域或浏览器执行。`apply_to_current_worker` 会不可逆地降低当前进程资源上限；宿主/UI 进程不得调用。先校验参数与继承限制，再安装并逐项读回确认；失败可能发生在部分限制已经安装后，worker 必须退出，不能忽略错误继续运行。

| 输入/输出 | 具体含义 |
| --- | --- |
| `additionalAddressBytes` | 启动时实际虚拟地址映射基线之外可新增的地址额度，16 MiB–64 GiB |
| `cpuSeconds` | 整个进程累计的 CPU 秒数，1–86,400 秒，不是逐个请求重置的墙钟超时 |
| `openFiles` | 进程文件描述符上限，3–4,096 |
| `startupVirtualBytes` | 当前 worker 在接收素材之前测到的真实映射基线 |
| `addressCeilingBytes` | 实际安装的绝对虚拟地址上限，且不高于继承的 soft/hard 限制 |
| core/regular file bytes | 固定为零；关闭 core dump 和普通文件增长，结果仍经宿主 pipe 传回 |

安装前还将 SIGXCPU/SIGXFSZ 恢复默认终止行为，并在启动线程解除屏蔽，避免继承宿主的 ignored/blocked 策略导致额度信号失效；后续 codec 线程继承这个 mask。当前使用 `RLIMIT_AS/CPU/NOFILE/CORE/FSIZE`；所有 soft/hard ceiling 均不提高继承限制，禁止以修改进程额度绕过宿主约束。CPU 超限可能产生 SIGXCPU 或在 hard ceiling 直接终止；文件增长可能失败并产生 SIGXFSZ，宿主必须把这种退出当作操作失败。

`RLIMIT_FSIZE=0` **不是文件系统沙箱**，它不拒绝读取已有文件，也不保证阻止删除、改名或原大小内写入。此模块也不控制网络、进程派生和系统调用权限。这些是后续平台沙箱/启动方案的独立责任，不能借“worker”或“rlimit”的名字混淆。

## 为什么不用固定总地址空间当内存限额

在本机实际加载固定 FFmpeg 组件的最小进程中，约 415 GiB 虚拟映射对应约 6 MiB 驻留内存；固定总地址空间 1 GiB 的 `setrlimit` 返回 EINVAL。Apple 的 [XNU 实现](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_resource.c)也明确在当前映射已超请求上限时拒绝降低。

因此启动时先测当前映射（macOS `proc_pidinfo(PROC_PIDTASKINFO)`；Linux `/proc/self/statm` 乘实际页大小），再增加明确的地址增长额度。无法读取基线时失败，不忽略限制。已实际验证“基线 + 1 GiB”能拒绝后续 2 GiB 的大分配；Rust 子进程测试另验证“基线 + 32 MiB”拒绝 64 MiB reserve，同时 64 KiB 正常分配仍成功。

该策略限制新增虚拟映射，**不等于 RSS/physical footprint 硬限额**。启动前已经保留的 allocator arena 仍可能增长其实际驻留页。产品还需要峰值内存观测、帧/缓存/解码工作预算与取消/墙钟期限，必要时由平台机制约束实际内存；不得把这个地址额度在 UI 或验收报告中标成“最大占用内存”。Linux 地址空间限制的系统语义见 [Linux man-pages](https://man7.org/linux/man-pages/man2/getrlimit.2.html)；本次只运行了本机 macOS 的实际测试。

## 依赖与验证

- 固定 rustix 1.1.5，扩展使用 `process/param` 功能，许可与来源见[记录](../../components/rustix/component.json)。
- macOS 基线查询及 Unix 限额信号设置使用已锁定的 libc 0.2.189（此前为间接依赖），限制在两个私有 FFI 模块中，进程信息校验返回完整结构长度后才读取；[来源与许可](../../components/libc/component.json)。
- 非 macOS/Linux 平台返回 `Unsupported`，不得静默不设限制。Windows Job Object 等仍待实现。
- 六项父测试均在独立子进程中安装限制（CPU 测试额外包含继承 ignored + blocked 信号的子场景），绝不更改 Cargo 测试主进程或用户程序。分别验证地址增长、CPU 内核终止、继承限制、无效输入不改变上限、额外文件描述符被内核拒绝，以及文件增长失败及 pipe 可写。
- ignored 的 child fixture 由六项父测试显式启动，不是未执行的功能测试。父进程有 15 秒墙钟兜底，异常时 kill/reap；不会遗留耗 CPU 的测试子进程。

```sh
cargo test --offline --locked -p mo-native-limits -- --nocapture
```

本机记录 `.codex-work/media-process-limits-evidence-final.log` 含各子进程实际安装值、退出信号与观察时间。两个 CPU 子场景均约 1.02 秒后收到 SIGXCPU；文件增长收到 SIGXFSZ，pipe 输出此前已成功。该时间是本次测试观察值，不是跨设备性能保证。早期测试曾错误地只接受普通错误返回而忽略合法 SIGXFSZ，现已按真实内核行为修正断言，原失败日志保留。

[视频 worker](video-media-worker.md)已接入启动限制，并实际验证宿主总调用 deadline、双向资源读取、frame payload 上限、取消/崩溃后的 kill/reap 与依赖闭包。该 worker 的证据单独记录；资源限制本身仍不构成系统调用沙箱。
