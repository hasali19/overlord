Param($CertPath, [securestring] $Password)

cargo build --release

Remove-Item -Recurse .\package
New-Item -ItemType Directory .\package

Copy-Item .\AppxManifest.xml .\package
Copy-Item .\target\release\overlord.exe .\package
Copy-Item .\images\* .\package

$PlainPassword = ConvertFrom-SecureString -SecureString $Password -AsPlainText;

& 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\makeappx.exe' pack /d .\package /p .\package\overlord
& 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\signtool.exe' sign /fd SHA256 /f $CertPath /p $PlainPassword .\package\overlord.msix
