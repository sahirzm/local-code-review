import { Dialog } from '@base-ui-components/react/dialog';
import { motion, AnimatePresence } from 'motion/react';
import type { ReactNode } from 'react';

interface ModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Accessible name for the dialog; mirrors the previous aria-label on the overlay. */
  ariaLabel: string;
  /** Dialog-box class (e.g. `modal-dialog`, `discard-dialog`); combined with centering. */
  dialogClassName: string;
  children: ReactNode;
}

/**
 * Thin wrapper over Base UI Dialog that adds focus trapping, Escape-to-close,
 * scroll locking, and a motion enter/exit while keeping each call site's dialog
 * markup. Base UI renders the backdrop and popup as siblings, so the popup owns
 * its own centering (`.modal-popup`) rather than relying on the backdrop's flex.
 */
export function Modal({ open, onOpenChange, ariaLabel, dialogClassName, children }: ModalProps): React.JSX.Element {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <AnimatePresence>
        {open && (
          <Dialog.Portal keepMounted>
            <Dialog.Backdrop
              render={
                <motion.div
                  className="modal-overlay"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.15 }}
                />
              }
            />
            <Dialog.Popup
              aria-label={ariaLabel}
              className="modal-popup"
              render={
                <motion.div
                  initial={{ opacity: 0, scale: 0.96, y: 8 }}
                  animate={{ opacity: 1, scale: 1, y: 0 }}
                  exit={{ opacity: 0, scale: 0.97, y: 4 }}
                  transition={{ duration: 0.18, ease: [0.16, 1, 0.3, 1] }}
                />
              }
            >
              <div className={dialogClassName}>{children}</div>
            </Dialog.Popup>
          </Dialog.Portal>
        )}
      </AnimatePresence>
    </Dialog.Root>
  );
}
