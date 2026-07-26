import React from 'react';

/** 1px hairline rule used to split panel sections. */
export function Hairline({ vertical = false, inset = 0, strong = false, style }) {
  const c = strong ? 'var(--line-hairline-strong)' : 'var(--line-hairline)';
  return <hr style={{ border: 0, margin: 0, alignSelf: 'stretch', ...(vertical ? { width: 1, background: c, marginBlock: inset } : { height: 1, background: c, marginInline: inset }), ...style }} />;
}
