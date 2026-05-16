import type { ParsedFileDiff, Hunk as OurHunk, Change as OurChange } from '../../../shared/types.js';
import type { FileData, HunkData, ChangeData } from 'react-diff-view';
import type { DiffType } from 'react-diff-view';

const STATUS_MAP: Record<ParsedFileDiff['status'], DiffType> = {
  added: 'add',
  deleted: 'delete',
  modified: 'modify',
  renamed: 'rename',
  copied: 'copy',
};

function transformChange(change: OurChange): ChangeData {
  switch (change.type) {
    case 'insert':
      return {
        type: 'insert',
        content: change.content,
        lineNumber: change.newLineNumber ?? 0,
        isInsert: true,
      };
    case 'delete':
      return {
        type: 'delete',
        content: change.content,
        lineNumber: change.oldLineNumber ?? 0,
        isDelete: true,
      };
    case 'normal':
      return {
        type: 'normal',
        content: change.content,
        oldLineNumber: change.oldLineNumber ?? 0,
        newLineNumber: change.newLineNumber ?? 0,
        isNormal: true,
      };
  }
}

function transformHunk(hunk: OurHunk): HunkData {
  return {
    content: hunk.content,
    oldStart: hunk.oldStart,
    newStart: hunk.newStart,
    oldLines: hunk.oldLines,
    newLines: hunk.newLines,
    changes: hunk.changes.map(transformChange),
  };
}

export function transformFile(file: ParsedFileDiff): FileData {
  return {
    oldPath: file.oldPath,
    newPath: file.newPath,
    type: STATUS_MAP[file.status],
    hunks: file.hunks.map(transformHunk),
    oldEndingNewLine: true,
    newEndingNewLine: true,
    oldMode: '',
    newMode: '',
    oldRevision: '',
    newRevision: '',
  };
}
