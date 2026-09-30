# Security

three-rs decodes untrusted input: PNG, JPEG, WebP, KTX2, HDR and glTF (with
Draco and meshopt compression). A crash, hang or out-of-bounds read on a
crafted file is a security bug.

Please report one privately through GitHub's
[private vulnerability reporting](https://github.com/oneilltomhq/three-rs/security/advisories/new),
not in a public issue. Include the file or a way to make it, and the version
of three-rs. You should hear back within a week. Fixes go out as a patch
release on the latest minor version.
