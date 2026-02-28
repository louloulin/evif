import { test as base, expect, Page, Locator } from '@playwright/test';
import { enableNewFileButton } from '../mocks/fixtures';

/**
 * Editor Page Object Model
 * Handles all interactions with the editor component
 */
export class EditorPage {
  readonly page: Page;
  readonly newFileButton: Locator;
  readonly editor: Locator;
  readonly breadcrumb: Locator;
  readonly editorContainer: Locator;
  readonly explorerButton: Locator;

  constructor(page: Page) {
    this.page = page;
    // The new file button contains "新建文件" text - initially disabled until explorer is opened
    this.newFileButton = page.getByRole('button', { name: /新建文件|new file/i });
    // Editor - Use monaco-editor specifically, not the container
    this.editor = page.locator('.monaco-editor');
    // Breadcrumb is the container with border-b bg-muted/30 class containing path buttons
    this.breadcrumb = page.locator('.border-b.bg-muted\\/30, [class*="border-b"][class*="bg-muted"]').first();
    this.editorContainer = page.locator('.editor-container, [data-testid="editor-container"]');
    this.explorerButton = page.getByRole('button', { name: /资源管理器|Explorer/i });
  }

  async goto() {
    await this.page.goto('/');
    await this.page.waitForLoadState('networkidle');

    // Enable new file button by mocking the file list state
    await enableNewFileButton(this.page);
  }

  async openExplorer() {
    // Click explorer button to enable new file button
    if (await this.explorerButton.isVisible()) {
      await this.explorerButton.click();
      await this.page.waitForTimeout(500);
    }
  }

  async clickNewFile() {
    // First ensure explorer is opened (new file button is disabled otherwise)
    await this.openExplorer();

    // Wait for the new file button to be enabled
    await this.page.waitForFunction(() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const newFileBtn = buttons.find(btn => btn.textContent?.includes('新建文件'));
      return newFileBtn && !newFileBtn.disabled;
    }, { timeout: 10000 }).catch(() => {});

    await this.newFileButton.waitFor({ state: 'visible', timeout: 10000 });

    // Additional wait to ensure button is clickable
    await this.page.waitForTimeout(500);

    await this.newFileButton.click();

    // Wait for editor to load after file creation
    await this.page.waitForTimeout(2000);
  }

  async saveFile() {
    // Save uses keyboard shortcut (Cmd+S or Ctrl+S) since there's no save button
    await this.saveFileWithShortcut();
  }

  async saveFileWithShortcut() {
    const isMac = await this.page.evaluate(() => navigator.platform.toUpperCase().indexOf('MAC') >= 0);
    if (isMac) {
      await this.page.keyboard.press('Meta+s');
    } else {
      await this.page.keyboard.press('Control+s');
    }
    await this.page.waitForTimeout(1000); // Wait for save to complete
  }

  async getEditorContent() {
    if (await this.editor.isVisible()) {
      return await this.editor.textContent();
    }
    return null;
  }

  async typeInEditor(text: string) {
    // Wait for Monaco editor to be fully loaded and interactive
    await this.editor.waitFor({ state: 'visible', timeout: 10000 });
    await this.page.waitForTimeout(500); // Additional wait for Monaco to initialize

    // Click on the editor to focus it
    await this.editor.click();

    // Wait for the cursor to be ready
    await this.page.waitForTimeout(300);

    await this.page.keyboard.type(text);
  }

  async waitForFileLoaded(filename: string) {
    await this.breadcrumb.waitFor({ state: 'visible', timeout: 5000 });
    await expect(this.breadcrumb).toContainText(filename);
  }

  async isNoFileOpenVisible() {
    const noFileText = this.editorContainer.locator('text=/No file open|没有打开文件|untitled/i');
    return await noFileText.isVisible().catch(() => false);
  }
}

/**
 * File Tree Page Object
 */
export class FileTreePage {
  readonly page: Page;
  readonly fileTree: Locator;
  readonly explorerButton: Locator;

  constructor(page: Page) {
    this.page = page;
    // The file tree has class "file-tree" and contains items with class "file-tree-item"
    this.fileTree = page.locator('.file-tree');
    this.explorerButton = page.locator('[data-testid="activity-explorer"], [title="Explorer"], [title="文件资源管理器"]');
  }

  async openExplorer() {
    if (await this.explorerButton.isVisible()) {
      await this.explorerButton.click();
      await this.fileTree.waitFor({ state: 'visible' });
    }
  }

  async expandFolder(folderName: string) {
    // Click on folder in file tree to expand it
    // Look for the file-tree-item with the folder name (has ▶ icon when collapsed)
    const folderItem = this.fileTree.locator('.file-tree-item').filter({ hasText: folderName }).first();
    if (await folderItem.isVisible().catch(() => false)) {
      await folderItem.click();
      await this.page.waitForTimeout(500); // Wait for expansion animation
    }
  }

  async clickFile(filename: string) {
    // File items have class "file-tree-item" and contain the filename in .file-name
    const fileItem = this.fileTree.locator('.file-tree-item').filter({ hasText: filename });
    await fileItem.click();
  }

  async rightClickFile(filename: string) {
    const fileItem = this.fileTree.locator('.file-tree-item').filter({ hasText: filename });
    await fileItem.click({ button: 'right' });
  }

  async isFileVisible(filename: string) {
    const fileItem = this.fileTree.locator('.file-tree-item').filter({ hasText: filename });
    return await fileItem.isVisible().catch(() => false);
  }

  async isFileVisibleExpanded(filename: string, parentFolder?: string) {
    // If parent folder specified, expand it first
    if (parentFolder) {
      await this.expandFolder(parentFolder);
    }
    return await this.isFileVisible(filename);
  }
}

/**
 * Search Page Object
 */
export class SearchPage {
  readonly page: Page;
  readonly searchButton: Locator;
  readonly searchInput: Locator;
  readonly searchResults: Locator;
  readonly filterPanel: Locator;

  constructor(page: Page) {
    this.page = page;
    this.searchButton = page.getByRole('button', { name: /search|搜索/i });
    this.searchInput = page.locator('[data-testid="search-input"], input[type="search"], input[placeholder*="search"], input[placeholder*="搜索"]');
    this.searchResults = page.locator('[data-testid="search-results"], .search-results');
    this.filterPanel = page.locator('[data-testid="filter-panel"], .filter-panel');
  }

  async openSearch() {
    if (await this.searchButton.isVisible()) {
      await this.searchButton.click();
    }
    await this.searchInput.waitFor({ state: 'visible' });
  }

  async search(keyword: string) {
    await this.searchInput.fill(keyword);
    await this.searchInput.press('Enter');
    await this.page.waitForTimeout(500);
  }

  async getResultsCount() {
    const countText = this.searchResults.locator('text=/\\d+ (result|结果)/i');
    if (await countText.isVisible()) {
      const text = await countText.textContent();
      const match = text?.match(/(\d+)/);
      return match ? parseInt(match[1]) : 0;
    }
    return 0;
  }

  async openFilterPanel() {
    const filterButton = this.page.getByRole('button', { name: /filter|过滤/i });
    if (await filterButton.isVisible()) {
      await filterButton.click();
      await this.filterPanel.waitFor({ state: 'visible' });
    }
  }
}

/**
 * Upload Page Object
 */
export class UploadPage {
  readonly page: Page;
  readonly dropzone: Locator;
  readonly fileInput: Locator;
  readonly uploadButton: Locator;
  readonly progressBar: Locator;

  constructor(page: Page) {
    this.page = page;
    this.dropzone = page.locator('[data-testid="upload-dropzone"], .upload-dropzone, [data-testid="dropzone"]');
    this.fileInput = page.locator('input[type="file"]');
    this.uploadButton = page.getByRole('button', { name: /upload|上传|选择文件/i });
    this.progressBar = page.locator('[data-testid="progress-bar"], .progress-bar, [role="progressbar"]');
  }

  async uploadFile(filePath: string) {
    if (await this.dropzone.isVisible()) {
      const fileInput = this.dropzone.locator('input[type="file"]');
      await fileInput.setInputFiles(filePath);
    } else if (await this.uploadButton.isVisible()) {
      await this.uploadButton.click();
      await this.fileInput.setInputFiles(filePath);
    }
  }

  async isProgressBarVisible() {
    return await this.progressBar.isVisible().catch(() => false);
  }

  async getProgressPercentage() {
    const progressText = this.progressBar.locator('text=/\\d+%/');
    if (await progressText.isVisible()) {
      const text = await progressText.textContent();
      const match = text?.match(/(\d+)/);
      return match ? parseInt(match[1]) : 0;
    }
    return 0;
  }
}

// Export fixtures for use in tests
export { expect };
