use std::{
    io::stdout,
    path::PathBuf,
    process::Command,
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

#[derive(Clone, PartialEq)]
enum InstallItem {
    Winget,
    NetBird,
}

#[derive(Clone, PartialEq)]
enum AppState {
    Menu,
    Installing(InstallItem),
    FileBrowser,
    Restoring,
    Result { success: bool, message: String },
}

struct App {
    state: AppState,
    menu_state: ListState,
    menu_items: Vec<&'static str>,
    log_messages: Vec<String>,
    // File browser
    current_dir: PathBuf,
    dir_entries: Vec<PathBuf>,
    file_list_state: ListState,
    selected_file: Option<PathBuf>,
}

impl App {
    fn new() -> Self {
        let mut menu_state = ListState::default();
        menu_state.select(Some(0));
        
        let default_dir = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\"))
            .join("ServerBackups");
        
        Self {
            state: AppState::Menu,
            menu_state,
            menu_items: vec![
                "Check Winget Status",
                "Install Winget",
                "Check NetBird Status",
                "Install NetBird",
                "Backup Server Roles & Features",
                "Restore Server Roles & Features",
                "Exit",
            ],
            log_messages: Vec::new(),
            current_dir: default_dir,
            dir_entries: Vec::new(),
            file_list_state: ListState::default(),
            selected_file: None,
        }
    }

    fn next(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i >= self.menu_items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.menu_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    fn add_log(&mut self, msg: impl Into<String>) {
        self.log_messages.push(msg.into());
    }

    fn check_winget_status(&self) -> (bool, String) {
        match Command::new("winget").arg("--version").output() {
            Ok(output) => {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    (true, format!("Winget is installed: {}", version.trim()))
                } else {
                    (false, "Winget is not working properly".to_string())
                }
            }
            Err(_) => (false, "Winget is not installed".to_string()),
        }
    }

    fn install_winget(&mut self) -> (bool, String) {
        self.log_messages.clear();
        self.add_log("Starting Winget installation for Windows Server...");

        // Create temp directory
        let temp_dir = std::env::temp_dir().join("winget_install");
        if let Err(e) = std::fs::create_dir_all(&temp_dir) {
            return (false, format!("Failed to create temp directory: {}", e));
        }

        self.add_log("Downloading required packages...");

        // URLs for required components
        let downloads = [
            (
                "Microsoft.VCLibs.x64.14.00.Desktop.appx",
                "https://aka.ms/Microsoft.VCLibs.x64.14.00.Desktop.appx"
            ),
            (
                "Microsoft.UI.Xaml.2.8.x64.appx",
                "https://github.com/nickel-org/nickel.rs/releases/download/0.0.0/Microsoft.UI.Xaml.2.8.x64.appx"
            ),
        ];

        // Download VCLibs
        self.add_log("Downloading Microsoft.VCLibs...");
        let vclibs_path = temp_dir.join("Microsoft.VCLibs.x64.14.00.Desktop.appx");
        
        let download_result = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                    downloads[0].1,
                    vclibs_path.display()
                )
            ])
            .output();

        if let Err(e) = download_result {
            return (false, format!("Failed to download VCLibs: {}", e));
        }

        // Download UI.Xaml from NuGet
        self.add_log("Downloading Microsoft.UI.Xaml...");
        let xaml_nupkg_path = temp_dir.join("microsoft.ui.xaml.2.8.6.nupkg");
        let xaml_result = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri 'https://www.nuget.org/api/v2/package/Microsoft.UI.Xaml/2.8.6' -OutFile '{}'",
                    xaml_nupkg_path.display()
                )
            ])
            .output();

        if let Err(e) = xaml_result {
            return (false, format!("Failed to download UI.Xaml: {}", e));
        }

        // Extract UI.Xaml
        self.add_log("Extracting Microsoft.UI.Xaml...");
        let xaml_extract_dir = temp_dir.join("xaml_extract");
        let _ = std::fs::create_dir_all(&xaml_extract_dir);
        
        let extract_result = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    xaml_nupkg_path.display(),
                    xaml_extract_dir.display()
                )
            ])
            .output();

        if let Err(e) = extract_result {
            return (false, format!("Failed to extract UI.Xaml: {}", e));
        }

        let xaml_appx_path = xaml_extract_dir.join("tools").join("AppX").join("x64").join("Release").join("Microsoft.UI.Xaml.2.8.appx");

        // Download Winget
        self.add_log("Downloading Winget...");
        let winget_path = temp_dir.join("Microsoft.DesktopAppInstaller.msixbundle");
        let winget_result = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri 'https://github.com/microsoft/winget-cli/releases/latest/download/Microsoft.DesktopAppInstaller_8wekyb3d8bbwe.msixbundle' -OutFile '{}'",
                    winget_path.display()
                )
            ])
            .output();

        if let Err(e) = winget_result {
            return (false, format!("Failed to download Winget: {}", e));
        }

        // Download license
        self.add_log("Downloading license...");
        let license_path = temp_dir.join("license.xml");
        let _license_result = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri 'https://github.com/microsoft/winget-cli/releases/latest/download/b]_License1.xml' -OutFile '{}'",
                    license_path.display()
                )
            ])
            .output();

        // Install packages
        self.add_log("Installing Microsoft.VCLibs...");
        let vclibs_install = Command::new("powershell")
            .args([
                "-Command",
                &format!("Add-AppxPackage -Path '{}'", vclibs_path.display())
            ])
            .output();

        if let Err(e) = vclibs_install {
            self.add_log(format!("Warning: VCLibs install issue: {}", e));
        }

        self.add_log("Installing Microsoft.UI.Xaml...");
        if xaml_appx_path.exists() {
            let xaml_install = Command::new("powershell")
                .args([
                    "-Command",
                    &format!("Add-AppxPackage -Path '{}'", xaml_appx_path.display())
                ])
                .output();

            if let Err(e) = xaml_install {
                self.add_log(format!("Warning: UI.Xaml install issue: {}", e));
            }
        }

        self.add_log("Installing Winget...");
        let winget_install = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Add-AppxPackage -Path '{}'",
                    winget_path.display()
                )
            ])
            .output();

        match winget_install {
            Ok(output) => {
                if output.status.success() {
                    self.add_log("Installation completed!");
                    
                    // Verify installation
                    std::thread::sleep(Duration::from_secs(2));
                    let (installed, msg) = self.check_winget_status();
                    if installed {
                        (true, format!("Winget installed successfully!\n{}", msg))
                    } else {
                        (true, "Installation completed. You may need to restart your terminal or system.".to_string())
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    (false, format!("Installation failed: {}", stderr))
                }
            }
            Err(e) => (false, format!("Failed to install Winget: {}", e)),
        }
    }

    fn check_netbird_status(&self) -> (bool, String) {
        match Command::new("netbird").arg("version").output() {
            Ok(output) => {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    (true, format!("NetBird is installed: {}", version.trim()))
                } else {
                    (false, "NetBird is not working properly".to_string())
                }
            }
            Err(_) => {
                // Also check in Program Files
                let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
                let netbird_path = std::path::Path::new(&program_files).join("NetBird").join("netbird.exe");
                if netbird_path.exists() {
                    (true, format!("NetBird is installed at: {}", netbird_path.display()))
                } else {
                    (false, "NetBird is not installed".to_string())
                }
            }
        }
    }

    fn install_netbird(&mut self) -> (bool, String) {
        self.log_messages.clear();
        self.add_log("Starting NetBird installation...");

        // First check if winget is available
        let (winget_available, _) = self.check_winget_status();
        
        if winget_available {
            self.add_log("Using winget to install NetBird...");
            
            let install_result = Command::new("winget")
                .args(["install", "--id", "NetBird.NetBird", "-e", "--accept-source-agreements", "--accept-package-agreements"])
                .output();

            match install_result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    
                    if output.status.success() || stdout.contains("Successfully installed") {
                        self.add_log("NetBird installed successfully!");
                        (true, format!("NetBird installed successfully via winget!\n\nTo connect, run:\n  netbird up"))
                    } else if stdout.contains("already installed") {
                        (true, "NetBird is already installed.".to_string())
                    } else {
                        (false, format!("Installation may have failed:\n{}\n{}", stdout, stderr))
                    }
                }
                Err(e) => (false, format!("Failed to run winget: {}", e)),
            }
        } else {
            // Fallback to PowerShell script installation
            self.add_log("Winget not available, using PowerShell installer...");
            
            let install_result = Command::new("powershell")
                .args([
                    "-ExecutionPolicy", "Bypass",
                    "-Command",
                    "Invoke-WebRequest -Uri 'https://github.com/netbirdio/netbird/releases/latest/download/netbird_installer_windows_amd64.exe' -OutFile '$env:TEMP\\netbird_installer.exe'; Start-Process -FilePath '$env:TEMP\\netbird_installer.exe' -ArgumentList '/S' -Wait"
                ])
                .output();

            match install_result {
                Ok(output) => {
                    if output.status.success() {
                        std::thread::sleep(Duration::from_secs(3));
                        let (installed, msg) = self.check_netbird_status();
                        if installed {
                            (true, format!("NetBird installed successfully!\n{}\n\nTo connect, run:\n  netbird up", msg))
                        } else {
                            (true, "Installation completed. You may need to restart your terminal.".to_string())
                        }
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        (false, format!("Installation failed: {}", stderr))
                    }
                }
                Err(e) => (false, format!("Failed to install NetBird: {}", e)),
            }
        }
    }

    fn backup_server_roles(&mut self) -> (bool, String) {
        self.log_messages.clear();
        self.add_log("Backing up Server Roles and Features...");

        // Create backup directory
        let backup_dir = dirs::document_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("C:\\ServerBackups"))
            .join("ServerBackups");
        
        if let Err(e) = std::fs::create_dir_all(&backup_dir) {
            return (false, format!("Failed to create backup directory: {}", e));
        }

        // Generate timestamp for filename
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let backup_file = backup_dir.join(format!("ServerRoles_{}.xml", timestamp));
        let features_file = backup_dir.join(format!("InstalledFeatures_{}.txt", timestamp));

        self.add_log("Exporting installed roles and features...");

        // Export Windows Features to XML (can be used for restoration)
        let export_result = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Get-WindowsFeature | Where-Object {{$_.Installed -eq $true}} | Export-Clixml -Path '{}'",
                    backup_file.display()
                )
            ])
            .output();

        if let Err(e) = export_result {
            return (false, format!("Failed to export roles: {}", e));
        }

        // Also create a human-readable list
        let list_result = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Get-WindowsFeature | Where-Object {{$_.Installed -eq $true}} | Select-Object Name, DisplayName, FeatureType | Format-Table -AutoSize | Out-File -FilePath '{}' -Width 200",
                    features_file.display()
                )
            ])
            .output();

        if let Err(e) = list_result {
            self.add_log(format!("Warning: Could not create readable list: {}", e));
        }

        // Verify the backup was created
        if backup_file.exists() {
            let metadata = std::fs::metadata(&backup_file);
            let size = metadata.map(|m| m.len()).unwrap_or(0);
            
            if size > 0 {
                (true, format!(
                    "Server Roles and Features backed up successfully!\n\n\
                    Backup location:\n  {}\n\n\
                    Readable list:\n  {}\n\n\
                    To restore on another server, use:\n  \
                    Import-Clixml '{}' | Where-Object {{$_.Installed}} | Install-WindowsFeature",
                    backup_file.display(),
                    features_file.display(),
                    backup_file.display()
                ))
            } else {
                (false, "Backup file was created but appears empty. Ensure you have admin rights.".to_string())
            }
        } else {
            (false, "Failed to create backup file. Ensure you are running as Administrator.".to_string())
        }
    }

    fn load_directory(&mut self) {
        self.dir_entries.clear();
        
        // Add parent directory option if not at root
        if let Some(parent) = self.current_dir.parent() {
            if parent.as_os_str().len() > 0 {
                self.dir_entries.push(PathBuf::from(".."));
            }
        }
        
        // Read directory contents
        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            let mut dirs: Vec<PathBuf> = Vec::new();
            let mut files: Vec<PathBuf> = Vec::new();
            
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().map(|e| e == "xml").unwrap_or(false) {
                    files.push(path);
                }
            }
            
            // Sort alphabetically
            dirs.sort();
            files.sort();
            
            // Add directories first, then XML files
            self.dir_entries.extend(dirs);
            self.dir_entries.extend(files);
        }
        
        // Select first item if available
        if !self.dir_entries.is_empty() {
            self.file_list_state.select(Some(0));
        } else {
            self.file_list_state.select(None);
        }
    }

    fn file_browser_next(&mut self) {
        if self.dir_entries.is_empty() {
            return;
        }
        let i = match self.file_list_state.selected() {
            Some(i) => {
                if i >= self.dir_entries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.file_list_state.select(Some(i));
    }

    fn file_browser_previous(&mut self) {
        if self.dir_entries.is_empty() {
            return;
        }
        let i = match self.file_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.dir_entries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.file_list_state.select(Some(i));
    }

    fn file_browser_select(&mut self) -> Option<PathBuf> {
        if let Some(i) = self.file_list_state.selected() {
            if let Some(path) = self.dir_entries.get(i) {
                if path == &PathBuf::from("..") {
                    // Go to parent directory
                    if let Some(parent) = self.current_dir.parent() {
                        self.current_dir = parent.to_path_buf();
                        self.load_directory();
                    }
                    return None;
                } else if path.is_dir() {
                    // Enter directory
                    self.current_dir = path.clone();
                    self.load_directory();
                    return None;
                } else {
                    // Select file
                    return Some(path.clone());
                }
            }
        }
        None
    }

    fn restore_server_roles(&mut self, backup_file: &PathBuf) -> (bool, String) {
        self.log_messages.clear();
        self.add_log(format!("Restoring from: {}", backup_file.display()));

        // Verify file exists
        if !backup_file.exists() {
            return (false, format!("Backup file not found: {}", backup_file.display()));
        }

        self.add_log("Reading backup file...");
        
        // First, let's see what features will be installed
        let preview_result = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "$features = Import-Clixml -Path '{}'; $features | Where-Object {{$_.Installed -eq $true}} | Select-Object -ExpandProperty Name",
                    backup_file.display()
                )
            ])
            .output();

        let features_list = match preview_result {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(e) => return (false, format!("Failed to read backup file: {}", e)),
        };

        self.add_log("Installing server roles and features...");
        self.add_log("This may take several minutes...");

        // Perform the actual restore
        let restore_result = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "$features = Import-Clixml -Path '{}'; \
                    $toInstall = $features | Where-Object {{$_.Installed -eq $true}} | Select-Object -ExpandProperty Name; \
                    if ($toInstall) {{ \
                        Install-WindowsFeature -Name $toInstall -IncludeManagementTools -ErrorAction SilentlyContinue | Out-String \
                    }} else {{ \
                        'No features to install' \
                    }}",
                    backup_file.display()
                )
            ])
            .output();

        match restore_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                if output.status.success() {
                    let restart_needed = stdout.contains("RestartNeeded") && stdout.contains("Yes");
                    let restart_msg = if restart_needed {
                        "\n\n⚠️  A system restart is required to complete the installation."
                    } else {
                        ""
                    };
                    
                    (true, format!(
                        "Server Roles and Features restoration completed!\n\n\
                        Features processed:\n{}\n\
                        Output:\n{}{}",
                        features_list.trim(),
                        stdout.trim(),
                        restart_msg
                    ))
                } else {
                    (false, format!(
                        "Restoration encountered errors:\n{}\n{}",
                        stdout.trim(),
                        stderr.trim()
                    ))
                }
            }
            Err(e) => (false, format!("Failed to execute restore: {}", e)),
        }
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match &app.state {
                        AppState::Menu => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Down | KeyCode::Char('j') => app.next(),
                            KeyCode::Up | KeyCode::Char('k') => app.previous(),
                            KeyCode::Enter => {
                                match app.menu_state.selected() {
                                    Some(0) => {
                                        let (success, message) = app.check_winget_status();
                                        app.state = AppState::Result { success, message };
                                    }
                                    Some(1) => {
                                        app.state = AppState::Installing(InstallItem::Winget);
                                    }
                                    Some(2) => {
                                        let (success, message) = app.check_netbird_status();
                                        app.state = AppState::Result { success, message };
                                    }
                                    Some(3) => {
                                        app.state = AppState::Installing(InstallItem::NetBird);
                                    }
                                    Some(4) => {
                                        let (success, message) = app.backup_server_roles();
                                        app.state = AppState::Result { success, message };
                                    }
                                    Some(5) => {
                                        // Open file browser for restore
                                        app.load_directory();
                                        app.state = AppState::FileBrowser;
                                    }
                                    Some(6) => return Ok(()),
                                    _ => {}
                                }
                            }
                            _ => {}
                        },
                        AppState::FileBrowser => match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.state = AppState::Menu;
                            }
                            KeyCode::Down | KeyCode::Char('j') => app.file_browser_next(),
                            KeyCode::Up | KeyCode::Char('k') => app.file_browser_previous(),
                            KeyCode::Enter => {
                                if let Some(file) = app.file_browser_select() {
                                    app.selected_file = Some(file);
                                    app.state = AppState::Restoring;
                                }
                            }
                            KeyCode::Backspace => {
                                // Go to parent directory
                                if let Some(parent) = app.current_dir.parent() {
                                    app.current_dir = parent.to_path_buf();
                                    app.load_directory();
                                }
                            }
                            _ => {}
                        },
                        AppState::Restoring => {
                            // Restoration will be handled in the draw loop
                        }
                        AppState::Installing(_) => {
                            // Installation will be handled in the draw loop
                        }
                        AppState::Result { .. } => match key.code {
                            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                                app.state = AppState::Menu;
                            }
                            _ => {}
                        },
                    }
                }
            }
        }

        // Handle installation state
        if let AppState::Installing(ref item) = app.state.clone() {
            let (title, msg) = match item {
                InstallItem::Winget => (" Installing Winget ", "Installing Winget... Please wait.\n\nThis may take a few minutes."),
                InstallItem::NetBird => (" Installing NetBird ", "Installing NetBird... Please wait.\n\nThis may take a few minutes."),
            };
            
            terminal.draw(|f| {
                let area = f.area();
                let block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow));
                let inner = block.inner(area);
                f.render_widget(block, area);
                
                let text = Paragraph::new(msg)
                    .style(Style::default().fg(Color::Yellow))
                    .wrap(Wrap { trim: true });
                f.render_widget(text, inner);
            })?;

            let (success, message) = match item {
                InstallItem::Winget => app.install_winget(),
                InstallItem::NetBird => app.install_netbird(),
            };
            app.state = AppState::Result { success, message };
        }

        // Handle restoring state
        if app.state == AppState::Restoring {
            terminal.draw(|f| {
                let area = f.area();
                let block = Block::default()
                    .title(" Restoring Server Roles & Features ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow));
                let inner = block.inner(area);
                f.render_widget(block, area);
                
                let text = Paragraph::new("Restoring Server Roles and Features...\n\nThis may take several minutes. Please wait.")
                    .style(Style::default().fg(Color::Yellow))
                    .wrap(Wrap { trim: true });
                f.render_widget(text, inner);
            })?;

            if let Some(ref file) = app.selected_file.clone() {
                let (success, message) = app.restore_server_roles(file);
                app.state = AppState::Result { success, message };
            } else {
                app.state = AppState::Result {
                    success: false,
                    message: "No file selected.".to_string(),
                };
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new(" Server Helper - Winget Installer ")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    match &app.state {
        AppState::Menu => {
            let items: Vec<ListItem> = app
                .menu_items
                .iter()
                .map(|i| ListItem::new(*i).style(Style::default().fg(Color::White)))
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title(" Menu ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(list, chunks[1], &mut app.menu_state);
        }
        AppState::Installing(ref item) => {
            let msg = match item {
                InstallItem::Winget => "Installing Winget... Please wait.",
                InstallItem::NetBird => "Installing NetBird... Please wait.",
            };
            let text = Paragraph::new(msg)
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .title(" Installing ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(text, chunks[1]);
        }
        AppState::FileBrowser => {
            let items: Vec<ListItem> = app
                .dir_entries
                .iter()
                .map(|path| {
                    let display = if path == &PathBuf::from("..") {
                        "📁 ..".to_string()
                    } else if path.is_dir() {
                        format!("📁 {}", path.file_name().unwrap_or_default().to_string_lossy())
                    } else {
                        format!("📄 {}", path.file_name().unwrap_or_default().to_string_lossy())
                    };
                    let style = if path.is_dir() || path == &PathBuf::from("..") {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(display).style(style)
                })
                .collect();

            let title = format!(" Select Backup File - {} ", app.current_dir.display());
            let list = List::new(items)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Magenta)),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Magenta)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(list, chunks[1], &mut app.file_list_state);
        }
        AppState::Restoring => {
            let text = Paragraph::new("Restoring Server Roles and Features...\n\nThis may take several minutes.")
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .title(" Restoring ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(text, chunks[1]);
        }
        AppState::Result { success, message } => {
            let (color, title) = if *success {
                (Color::Green, " Success ")
            } else {
                (Color::Red, " Error ")
            };

            let text = Paragraph::new(message.as_str())
                .style(Style::default().fg(color))
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(color)),
                )
                .wrap(Wrap { trim: true });
            f.render_widget(text, chunks[1]);
        }
    }

    // Footer
    let footer_text = match app.state {
        AppState::Menu => "↑/↓: Navigate | Enter: Select | q: Quit",
        AppState::FileBrowser => "↑/↓: Navigate | Enter: Select/Open | Backspace: Parent | Esc: Cancel",
        AppState::Installing(_) | AppState::Restoring => "Please wait...",
        AppState::Result { .. } => "Press Enter or Esc to return to menu",
    };
    
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}
