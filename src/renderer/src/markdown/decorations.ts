import { Decoration } from '@codemirror/view';
import { BulletWidget } from './widgets';

export const hiddenSyntax = Decoration.mark({ class: 'cm-markdown-syntax-hidden' });
export const hiddenHeadingSyntax = Decoration.replace({});
export const liveStrong = Decoration.mark({ class: 'cm-live-strong' });
export const liveEmphasis = Decoration.mark({ class: 'cm-live-emphasis' });
export const liveStrikethrough = Decoration.mark({ class: 'cm-live-strikethrough' });
export const liveLink = Decoration.mark({ class: 'cm-live-link' });
export const liveCode = Decoration.mark({ class: 'cm-live-code' });
export const liveCheckedTask = Decoration.mark({ class: 'cm-live-task-checked' });
export const bulletMarker = Decoration.replace({ widget: new BulletWidget() });
export const tablePreviewLine = Decoration.line({ class: 'cm-live-table-line' });
export const hiddenTableSourceLine = Decoration.line({ class: 'cm-live-table-source-hidden' });

export const fencedCodeLine = Decoration.line({ class: 'cm-live-codeblock' });
export const fencedCodeFirstLine = Decoration.line({ class: 'cm-live-codeblock cm-live-codeblock-first' });
export const fencedCodeLastLine = Decoration.line({ class: 'cm-live-codeblock cm-live-codeblock-last' });
export const fencedCodeSingleLine = Decoration.line({
  class: 'cm-live-codeblock cm-live-codeblock-first cm-live-codeblock-last'
});

export const headingClasses = [
  'cm-live-heading-1',
  'cm-live-heading-2',
  'cm-live-heading-3',
  'cm-live-heading-4',
  'cm-live-heading-5',
  'cm-live-heading-6'
];
