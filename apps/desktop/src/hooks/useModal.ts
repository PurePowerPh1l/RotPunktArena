import { useLayoutEffect, useRef } from "react";

const stack: HTMLElement[] = [];
const blocked = new Map<HTMLElement, { count: number; previous: boolean }>();
const focusable = 'button:not(:disabled),a[href],input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex]:not([tabindex="-1"])';

/** Shared native focus boundary. Render the returned ref in a body portal. */
export function useModal(enabled = true, onEscape?: () => void) {
  const ref = useRef<HTMLDivElement>(null);
  const escape = useRef(onEscape);
  escape.current = onEscape;
  useLayoutEffect(() => {
    const root = ref.current;
    if (!enabled || !root) return;
    const previousFocus = document.activeElement;
    const siblings = Array.from(document.body.children).filter((node): node is HTMLElement => node instanceof HTMLElement && node !== root);
    for (const node of siblings) {
      const state = blocked.get(node) ?? { count: 0, previous: node.inert };
      state.count += 1;
      blocked.set(node, state);
      node.inert = true;
    }
    root.inert = false;
    stack.push(root);
    root.tabIndex = -1;
    const elements = () => Array.from(root.querySelectorAll<HTMLElement>(focusable)).filter(node => node.getClientRects().length > 0);
    const initial = root.querySelector<HTMLElement>('[data-modal-initial]') ?? elements()[0] ?? root;
    initial.focus();
    const keydown = (event: KeyboardEvent) => {
      if (stack[stack.length - 1] !== root) return;
      if (event.key === "Escape" && escape.current) {
        event.preventDefault(); event.stopImmediatePropagation(); escape.current();
      }
      if (event.key !== "Tab") return;
      const nodes = elements();
      const first = nodes[0] ?? root;
      const last = nodes[nodes.length - 1] ?? root;
      if (event.shiftKey && (document.activeElement === first || !root.contains(document.activeElement))) {
        event.preventDefault(); last.focus();
      } else if (!event.shiftKey && (document.activeElement === last || !root.contains(document.activeElement))) {
        event.preventDefault(); first.focus();
      }
    };
    const focusin = (event: FocusEvent) => {
      if (stack[stack.length - 1] === root && !root.contains(event.target as Node)) (elements()[0] ?? root).focus();
    };
    document.addEventListener("keydown", keydown, true);
    document.addEventListener("focusin", focusin, true);
    return () => {
      document.removeEventListener("keydown", keydown, true);
      document.removeEventListener("focusin", focusin, true);
      stack.splice(stack.indexOf(root), 1);
      for (const node of siblings) {
        const state = blocked.get(node);
        if (state && --state.count === 0) { node.inert = state.previous; blocked.delete(node); }
      }
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected && !previousFocus.closest('[inert]')) previousFocus.focus();
    };
  }, [enabled]);
  return ref;
}
