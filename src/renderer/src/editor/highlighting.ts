import { HighlightStyle } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';

export const litheHighlightStyle = HighlightStyle.define([
  { tag: t.keyword, color: '#8f3f71' },
  { tag: [t.name, t.deleted, t.character, t.macroName], color: '#2f5f62' },
  { tag: [t.propertyName, t.variableName], color: '#245f9f' },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.labelName], color: '#7b4f18' },
  { tag: [t.color, t.constant(t.name), t.standard(t.name)], color: '#9a4b33' },
  { tag: [t.definition(t.name), t.separator], color: '#25211b' },
  { tag: [t.typeName, t.className, t.number, t.changed, t.annotation, t.modifier, t.self, t.namespace], color: '#6f4aa3' },
  { tag: [t.operator, t.operatorKeyword], color: '#4e6b6c' },
  { tag: [t.url, t.escape, t.regexp, t.link], color: '#186a75' },
  { tag: [t.meta, t.comment], color: '#7b756b', fontStyle: 'italic' },
  { tag: t.strong, fontWeight: '700' },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.strikethrough, textDecoration: 'line-through' },
  { tag: [t.atom, t.bool, t.special(t.variableName)], color: '#a04f16' },
  { tag: [t.processingInstruction, t.string, t.inserted], color: '#4f6f25' },
  { tag: t.invalid, color: '#b3261e' }
]);
