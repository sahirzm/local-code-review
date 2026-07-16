import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import {
  File,
  FileCode,
  FileJson,
  FileText,
  FileCog,
  FileTerminal,
  FileType,
  Database,
} from 'lucide-react';
import { getFileIconComponent, FileIcon } from '../file-icon.js';

describe('getFileIconComponent', () => {
  it('maps code extensions to the code icon', () => {
    for (const name of ['index.ts', 'App.tsx', 'main.js', 'mod.rs', 'app.py', 'Main.java']) {
      expect(getFileIconComponent(name)).toBe(FileCode);
    }
  });

  it('maps json to the json icon', () => {
    expect(getFileIconComponent('package.json')).toBe(FileJson);
  });

  it('maps config extensions to the cog icon', () => {
    for (const name of ['config.yaml', 'app.yml', 'Cargo.toml', 'settings.ini', '.env']) {
      expect(getFileIconComponent(name)).toBe(FileCog);
    }
  });

  it('maps shell scripts to the terminal icon', () => {
    expect(getFileIconComponent('deploy.sh')).toBe(FileTerminal);
    expect(getFileIconComponent('run.bash')).toBe(FileTerminal);
  });

  it('maps sql to the database icon', () => {
    expect(getFileIconComponent('schema.sql')).toBe(Database);
  });

  it('maps markup/style extensions to the type icon', () => {
    for (const name of ['styles.css', 'index.html', 'data.xml', 'logo.svg']) {
      expect(getFileIconComponent(name)).toBe(FileType);
    }
  });

  it('maps prose extensions to the text icon', () => {
    expect(getFileIconComponent('README.md')).toBe(FileText);
    expect(getFileIconComponent('notes.txt')).toBe(FileText);
  });

  it('is case-insensitive on the extension', () => {
    expect(getFileIconComponent('INDEX.TS')).toBe(FileCode);
    expect(getFileIconComponent('DATA.JSON')).toBe(FileJson);
  });

  it('uses the dotted segment even for dotfiles', () => {
    // ".env".split(".").pop() === "env" → cog icon
    expect(getFileIconComponent('.env')).toBe(FileCog);
  });

  it('falls back to the generic file icon for unknown/absent extensions', () => {
    expect(getFileIconComponent('Makefile')).toBe(File);
    expect(getFileIconComponent('LICENSE')).toBe(File);
    expect(getFileIconComponent('archive.zip')).toBe(File);
  });
});

describe('FileIcon', () => {
  it('renders an svg carrying the tree-file-icon class and aria-hidden', () => {
    const { container } = render(<FileIcon name="index.ts" />);
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute('class')).toContain('tree-file-icon');
    expect(svg!.getAttribute('aria-hidden')).toBe('true');
  });

  it('renders visually distinct icons for different file types', () => {
    const ts = render(<FileIcon name="index.ts" />).container.querySelector('svg')!.getAttribute('class');
    const json = render(<FileIcon name="package.json" />).container.querySelector('svg')!.getAttribute('class');
    const md = render(<FileIcon name="README.md" />).container.querySelector('svg')!.getAttribute('class');
    expect(ts).not.toBe(json);
    expect(ts).not.toBe(md);
    expect(json).not.toBe(md);
  });
});
