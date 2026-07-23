// Reading the live text selection and its surrounding context.

export interface SelectionSnapshot {
  quote: string;
  prefix: string;
  suffix: string;
  /** The selection's containing block, used as the fallback extraction root. */
  block: Element | null;
}

const CONTEXT = 40;

function containingBlock(node: Node): Element | null {
  const el = node.nodeType === Node.ELEMENT_NODE ? (node as Element) : node.parentElement;
  return el?.closest("article,main,section,p,li,blockquote,div") ?? null;
}

function contextAround(range: Range): { prefix: string; suffix: string } {
  const block = containingBlock(range.startContainer) ?? document.body;
  let prefix = "";
  let suffix = "";
  try {
    const pre = document.createRange();
    pre.selectNodeContents(block);
    pre.setEnd(range.startContainer, range.startOffset);
    prefix = pre.toString().slice(-CONTEXT);

    const post = document.createRange();
    post.selectNodeContents(block);
    post.setStart(range.endContainer, range.endOffset);
    suffix = post.toString().slice(0, CONTEXT);
  } catch {
    // Boundary math can throw on exotic selections; context is optional.
  }
  return { prefix, suffix };
}

export function readSelection(): SelectionSnapshot | null {
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || selection.rangeCount === 0) return null;
  const quote = selection.toString();
  if (!quote.trim()) return null;

  const range = selection.getRangeAt(0);
  const { prefix, suffix } = contextAround(range);
  return { quote, prefix, suffix, block: containingBlock(range.commonAncestorContainer) };
}

export function selectionRect(): DOMRect | null {
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || selection.rangeCount === 0) return null;
  const rect = selection.getRangeAt(0).getBoundingClientRect();
  if (rect.width === 0 && rect.height === 0) return null;
  return rect;
}
