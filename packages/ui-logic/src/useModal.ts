// ABOUTME: Headless modal hook for dialog state management
// ABOUTME: Shared open/close logic and escape key handling

import { useState, useCallback, useEffect, useRef } from 'react';

/**
 * Props for useModal hook
 */
export interface UseModalProps {
  /** Initial open state */
  initialOpen?: boolean;
  /** Callback when modal opens */
  onOpen?: () => void;
  /** Callback when modal closes */
  onClose?: () => void;
  /** Close on escape key press (web only, default: true) */
  closeOnEscape?: boolean;
  /** Close when clicking outside (default: true) */
  closeOnOutsideClick?: boolean;
}

/**
 * Return type for useModal hook
 */
export interface UseModalReturn {
  /** Whether modal is currently open */
  isOpen: boolean;
  /** Open the modal */
  open: () => void;
  /** Close the modal */
  close: () => void;
  /** Toggle modal state */
  toggle: () => void;
  /** Props for the modal backdrop/overlay */
  backdropProps: {
    onClick: (e: { target: unknown; currentTarget: unknown }) => void;
  };
  /** Props for the modal content container (stops propagation) */
  contentProps: {
    onClick: (e: { stopPropagation: () => void }) => void;
  };
  /** Ref callback for escape key handling (attach to document on web) */
  escapeKeyHandler: (e: { key: string }) => void;
}

/**
 * Headless modal hook
 *
 * Manages modal open/close state:
 * - Open/close/toggle functions
 * - Escape key handling
 * - Outside click handling
 * - Open/close callbacks
 *
 * @example
 * // Web usage
 * const modal = useModal({
 *   onClose: () => console.log('Modal closed'),
 *   closeOnEscape: true,
 * });
 *
 * useEffect(() => {
 *   if (modal.isOpen) {
 *     document.addEventListener('keydown', modal.escapeKeyHandler);
 *     return () => document.removeEventListener('keydown', modal.escapeKeyHandler);
 *   }
 * }, [modal.isOpen]);
 *
 * return modal.isOpen ? (
 *   <div className="backdrop" {...modal.backdropProps}>
 *     <div className="modal-content" {...modal.contentProps}>
 *       <button onClick={modal.close}>Close</button>
 *     </div>
 *   </div>
 * ) : null;
 *
 * @example
 * // Mobile usage
 * const modal = useModal({ onClose: handleClose });
 *
 * return (
 *   <Modal visible={modal.isOpen} onRequestClose={modal.close}>
 *     <View>
 *       <Button title="Close" onPress={modal.close} />
 *     </View>
 *   </Modal>
 * );
 */
export function useModal({
  initialOpen = false,
  onOpen,
  onClose,
  closeOnEscape = true,
  closeOnOutsideClick = true,
}: UseModalProps = {}): UseModalReturn {
  const [isOpen, setIsOpen] = useState(initialOpen);
  const wasOpenRef = useRef(initialOpen);

  // Track open state changes for callbacks
  useEffect(() => {
    if (isOpen && !wasOpenRef.current) {
      onOpen?.();
    } else if (!isOpen && wasOpenRef.current) {
      onClose?.();
    }
    wasOpenRef.current = isOpen;
  }, [isOpen, onOpen, onClose]);

  const open = useCallback(() => {
    setIsOpen(true);
  }, []);

  const close = useCallback(() => {
    setIsOpen(false);
  }, []);

  const toggle = useCallback(() => {
    setIsOpen(prev => !prev);
  }, []);

  const escapeKeyHandler = useCallback((e: { key: string }) => {
    if (closeOnEscape && e.key === 'Escape' && isOpen) {
      close();
    }
  }, [closeOnEscape, isOpen, close]);

  const backdropProps = {
    onClick: useCallback((e: { target: unknown; currentTarget: unknown }) => {
      // Only close if clicking the backdrop itself, not its children
      if (closeOnOutsideClick && e.target === e.currentTarget) {
        close();
      }
    }, [closeOnOutsideClick, close]),
  };

  const contentProps = {
    onClick: useCallback((e: { stopPropagation: () => void }) => {
      // Prevent clicks inside modal from closing it
      e.stopPropagation();
    }, []),
  };

  return {
    isOpen,
    open,
    close,
    toggle,
    backdropProps,
    contentProps,
    escapeKeyHandler,
  };
}

/**
 * Props for useConfirmDialog hook
 */
export interface UseConfirmDialogProps {
  /** Callback when confirmed */
  onConfirm: () => void | Promise<void>;
  /** Callback when cancelled */
  onCancel?: () => void;
  /** Close dialog after confirm action completes (default: true) */
  closeOnConfirm?: boolean;
}

/**
 * Return type for useConfirmDialog hook
 */
export interface UseConfirmDialogReturn extends UseModalReturn {
  /** Handle confirm action */
  confirm: () => Promise<void>;
  /** Handle cancel action */
  cancel: () => void;
  /** Whether confirm action is in progress */
  isConfirming: boolean;
}

/**
 * Headless confirm dialog hook
 *
 * Extends useModal with confirm/cancel actions:
 * - Async confirm support
 * - Loading state during confirm
 * - Auto-close on confirm
 *
 * @example
 * const deleteDialog = useConfirmDialog({
 *   onConfirm: async () => {
 *     await api.deleteItem(itemId);
 *     showToast('Deleted!');
 *   },
 * });
 *
 * return (
 *   <>
 *     <Button onClick={deleteDialog.open}>Delete</Button>
 *     {deleteDialog.isOpen && (
 *       <ConfirmDialog
 *         title="Delete Item?"
 *         onConfirm={deleteDialog.confirm}
 *         onCancel={deleteDialog.cancel}
 *         loading={deleteDialog.isConfirming}
 *       />
 *     )}
 *   </>
 * );
 */
export function useConfirmDialog({
  onConfirm,
  onCancel,
  closeOnConfirm = true,
}: UseConfirmDialogProps): UseConfirmDialogReturn {
  const modal = useModal();
  const [isConfirming, setIsConfirming] = useState(false);

  const confirm = useCallback(async () => {
    setIsConfirming(true);
    try {
      await onConfirm();
      if (closeOnConfirm) {
        modal.close();
      }
    } finally {
      setIsConfirming(false);
    }
  }, [onConfirm, closeOnConfirm, modal]);

  const cancel = useCallback(() => {
    onCancel?.();
    modal.close();
  }, [onCancel, modal]);

  return {
    ...modal,
    confirm,
    cancel,
    isConfirming,
  };
}
