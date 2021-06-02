fn main() {
    if cfg!(target_os = "windows") {
        windows::build!(
            Windows::Win32::Storage::FileSystem::GetDriveTypeW,
            Windows::Win32::Storage::FileSystem::GetLogicalDrives,
            Windows::Win32::System::SystemServices::PWSTR
        );
    }
}
