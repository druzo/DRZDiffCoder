# scripts/release/drz-app.spec
# RPM spec template for DRZ Diff. Placeholders (@VERSION@, @RELEASE@, @ARCH@,
# @PAYLOAD@, @ICON@, @LICENSE@) are interpolated by scripts/release/build-rpm.sh.
#
# Requires: rpm-build >= 4.19.
%global debug_package %{nil}

Name:           drzdiff
Version:        @VERSION@
Release:        @RELEASE@%{?dist}
Summary:        Source code diff/compare tool with tree-sitter highlighting
License:        MIT
URL:            https://github.com/druzo/DRZDiffCoder
Source0:        drzdiff-%{version}-%{arch}.tar.gz
ExclusiveArch:  @ARCH@

%description
DRZ Diff is a side-by-side source code diff/merge tool with language-aware
syntax highlighting (22 languages via tree-sitter) and inline editing.
Built with Rust + egui + tree-sitter.

%prep
# Payload is staged by build-rpm.sh; nothing to unpack.
%setup -T -c -n drzdiff-%{version}

%build
# No build step — binary is supplied pre-built.
:

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps
mkdir -p %{buildroot}/usr/share/icons/hicolor/128x128/apps
mkdir -p %{buildroot}/usr/share/icons/hicolor/48x48/apps
mkdir -p %{buildroot}/usr/share/doc/drzdiff

install -m755 @PAYLOAD@ %{buildroot}/usr/bin/drzdiff
install -m644 @ICON@ %{buildroot}/usr/share/icons/hicolor/256x256/apps/drzdiff.png
install -m644 @ICON@ %{buildroot}/usr/share/icons/hicolor/128x128/apps/drzdiff.png
install -m644 @ICON@ %{buildroot}/usr/share/icons/hicolor/48x48/apps/drzdiff.png
install -m644 @LICENSE@ %{buildroot}/usr/share/doc/drzdiff/LICENSE

cat > %{buildroot}/usr/share/applications/drzdiff.desktop <<EOF
[Desktop Entry]
Name=DRZ Diff
Comment=Source code diff comparer
Exec=drzdiff %U
Icon=drzdiff
Type=Application
Terminal=false
Categories=Development;Utility;
StartupWMClass=drzdiff
MimeType=text/plain;text/x-rust;text/x-python;text/x-c;text/x-c++;text/javascript;
EOF

%files
/usr/bin/drzdiff
/usr/share/applications/drzdiff.desktop
/usr/share/icons/hicolor/256x256/apps/drzdiff.png
/usr/share/icons/hicolor/128x128/apps/drzdiff.png
/usr/share/icons/hicolor/48x48/apps/drzdiff.png
/usr/share/doc/drzdiff/LICENSE

%changelog
* @RELEASE@ DRZ <noreply@drzdiff.local> - @VERSION@-@RELEASE@
- DRZ Diff release @VERSION@ (see docs/superpowers/specs/2026-08-07-v0.1.2-notes.md)