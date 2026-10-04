# Security policy

## Report a vulnerability privately

Use [GitHub private vulnerability reporting](https://github.com/wanquanY/MusterOffice/security/advisories/new).
Please do not disclose an unpatched vulnerability in a public issue or discussion.

Include the affected commit/version, platform, entry point, reproduction steps,
impact and a minimal sanitized input. Document parsing, native components,
resource bounds and file/process adapters are security-relevant surfaces.
Do not include live credentials, confidential presentations or other people's data.

The maintainer will assess the report and coordinate remediation and disclosure
through the private advisory. This early project does not promise a fixed response
time or maintain a supported stable-release series. Report against the latest
`main` when possible, while preserving the original affected revision.

## Supported scope

The current development branch receives fixes. Historical experimental bundles
are not supported release branches. MusterOffice CI, CodeQL, dependency review and
secret protection provide checks; they do not certify arbitrary inputs as safe.
Receiving products still own access control, worker isolation, final saving and
resource policies as described in the project architecture.

## 中文

请通过上方 GitHub 私密漏洞报告入口提交问题，不要在公开 Issue 中披露未修复漏洞。
附受影响版本、平台、入口、复现步骤、影响和脱敏后的最小输入，不上传真实密钥或私有文稿。
当前修复面向开发主分支；尚无稳定发行支持周期或固定响应时限承诺。
