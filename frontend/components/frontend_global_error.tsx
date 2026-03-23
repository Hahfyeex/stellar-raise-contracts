import React, { Component, ErrorInfo, ReactNode } from 'react';

/**
 * Props for the GlobalErrorBoundary component.
 */
interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

/**
 * State for the GlobalErrorBoundary component.
 */
interface State {
  hasError: boolean;
  error?: Error;
  errorInfo?: ErrorInfo;
  isSmartContractError: boolean;
}

/**
 * Global Error Boundary for handling React errors and smart contract errors.
 *
 * This component catches JavaScript errors anywhere in the component tree,
 * logs them, and displays a fallback UI instead of crashing the entire app.
 *
 * For smart contract errors, it provides specific handling and user-friendly
 * messages to improve UX when blockchain operations fail.
 *
 * @example
 * ```tsx
 * <GlobalErrorBoundary>
 *   <App />
 * </GlobalErrorBoundary>
 * ```
 */
class GlobalErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, isSmartContractError: false };
  }

  /**
   * Static method to update state when an error occurs.
   * This is called during the render phase, so it should be pure.
   *
   * @param error - The error that was thrown
   * @returns Updated state with error information
   */
  static getDerivedStateFromError(error: Error): State {
    // Check if this is a smart contract related error
    const isSmartContractError = GlobalErrorBoundary.isSmartContractError(error);

    return {
      hasError: true,
      error,
      isSmartContractError,
    };
  }

  /**
   * Lifecycle method called when an error is caught.
   * Used for side effects like logging errors.
   *
   * @param error - The error that was thrown
   * @param errorInfo - Additional error information from React
   */
  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    // Log error for debugging (in production, send to error reporting service)
    console.error('Global Error Boundary caught an error:', error, errorInfo);

    // Update state with error info for potential recovery
    this.setState({
      error,
      errorInfo,
    });

    // In a real app, you might want to send this to an error reporting service
    // like Sentry, LogRocket, or similar
    this.reportError(error, errorInfo);
  }

  /**
   * Determines if an error is related to smart contract operations.
   * Public for testing purposes.
   *
   * @param error - The error to check
   * @returns True if the error is smart contract related
   */
  public static isSmartContractError(error: Error): boolean {
    const errorMessage = error.message.toLowerCase();
    const errorName = error.name.toLowerCase();

    // Common smart contract error patterns
    const smartContractPatterns = [
      'contract',
      'stellar',
      'soroban',
      'transaction',
      'network',
      'blockchain',
      'timeout',
      'insufficient',
      'unauthorized',
      'invalid',
      'overflow',
      'underflow',
    ];

    return (
      smartContractPatterns.some(pattern =>
        errorMessage.includes(pattern) || errorName.includes(pattern)
      ) ||
      // Check for specific error types
      error instanceof ContractError ||
      error instanceof NetworkError ||
      error instanceof TransactionError
    );
  }

  /**
   * Reports the error to external services.
   * In production, this would send to error reporting services.
   *
   * @param error - The error that occurred
   * @param errorInfo - Additional React error info
   */
  private reportError(error: Error, errorInfo: ErrorInfo) {
    // Example: Send to error reporting service
    // In a real implementation, you might use:
    // - Sentry: Sentry.captureException(error, { contexts: { react: errorInfo } })
    // - LogRocket: LogRocket.captureException(error, { extra: errorInfo })
    // - Custom analytics service

    const errorReport = {
      message: error.message,
      stack: error.stack,
      componentStack: errorInfo.componentStack,
      timestamp: new Date().toISOString(),
      userAgent: typeof window !== 'undefined' ? window.navigator.userAgent : 'server',
      url: typeof window !== 'undefined' ? window.location.href : 'server',
      isSmartContractError: this.state.isSmartContractError,
    };

    // For now, just log to console. In production, send to service.
    console.error('Error Report:', errorReport);
  }

  /**
   * Attempts to recover from the error by resetting the error state.
   */
  private handleRetry = () => {
    this.setState({
      hasError: false,
      error: undefined,
      errorInfo: undefined,
      isSmartContractError: false,
    });
  };

  /**
   * Renders the error boundary.
   * If there's an error, shows the fallback UI; otherwise renders children.
   */
  render() {
    if (this.state.hasError) {
      // If a custom fallback is provided, use it
      if (this.props.fallback) {
        return this.props.fallback;
      }

      // Default error UI
      return <ErrorFallback
        error={this.state.error}
        isSmartContractError={this.state.isSmartContractError}
        onRetry={this.handleRetry}
      />;
    }

    return this.props.children;
  }
}

/**
 * Props for the ErrorFallback component.
 */
interface ErrorFallbackProps {
  error?: Error;
  isSmartContractError: boolean;
  onRetry: () => void;
}

/**
 * Default error fallback UI component.
 * Displays user-friendly error messages and recovery options.
 */
const ErrorFallback: React.FC<ErrorFallbackProps> = ({
  error,
  isSmartContractError,
  onRetry,
}) => {
  const getErrorMessage = () => {
    if (isSmartContractError) {
      return {
        title: 'Smart Contract Error',
        message: 'There was an issue with the blockchain transaction. This might be due to network congestion, insufficient funds, or a temporary service issue.',
        icon: '🔗',
      };
    }

    return {
      title: 'Something went wrong',
      message: 'An unexpected error occurred. Our team has been notified and is working to fix this issue.',
      icon: '⚠️',
    };
  };

  const { title, message, icon } = getErrorMessage();

  return (
    <div style={styles.container}>
      <div style={styles.content}>
        <div style={styles.icon}>{icon}</div>
        <h1 style={styles.title}>{title}</h1>
        <p style={styles.message}>{message}</p>

        {process.env.NODE_ENV === 'development' && error && (
          <details style={styles.errorDetails}>
            <summary style={styles.errorSummary}>Error Details (Development)</summary>
            <pre style={styles.errorText}>
              {error.name}: {error.message}
              {error.stack && `\n\n${error.stack}`}
            </pre>
          </details>
        )}

        <div style={styles.actions}>
          <button style={styles.primaryButton} onClick={onRetry}>
            Try Again
          </button>
          <button
            style={styles.secondaryButton}
            onClick={() => window.location.href = '/'}
          >
            Go Home
          </button>
        </div>
      </div>
    </div>
  );
};

/**
 * Custom error classes for better error categorization.
 */
export class ContractError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ContractError';
  }
}

export class NetworkError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'NetworkError';
  }
}

export class TransactionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'TransactionError';
  }
}

/**
 * Styles for the error boundary components.
 */
const styles = {
  container: {
    display: 'flex',
    justifyContent: 'center',
    alignItems: 'center',
    minHeight: '100vh',
    backgroundColor: '#f9fafb',
    fontFamily: 'system-ui, -apple-system, sans-serif',
    padding: '1rem',
  },
  content: {
    textAlign: 'center' as const,
    maxWidth: '500px',
    padding: '2rem',
    backgroundColor: 'white',
    borderRadius: '8px',
    boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1)',
  },
  icon: {
    fontSize: '3rem',
    marginBottom: '1rem',
  },
  title: {
    fontSize: '1.5rem',
    fontWeight: 'bold',
    color: '#1f2937',
    margin: '0 0 1rem 0',
  },
  message: {
    color: '#6b7280',
    margin: '0 0 1.5rem 0',
    lineHeight: '1.5',
  },
  errorDetails: {
    textAlign: 'left' as const,
    margin: '1rem 0',
    border: '1px solid #e5e7eb',
    borderRadius: '4px',
  },
  errorSummary: {
    padding: '0.5rem',
    cursor: 'pointer',
    backgroundColor: '#f3f4f6',
    borderRadius: '4px 4px 0 0',
  },
  errorText: {
    padding: '0.5rem',
    backgroundColor: '#f9fafb',
    borderRadius: '0 0 4px 4px',
    fontSize: '0.875rem',
    whiteSpace: 'pre-wrap' as const,
    wordBreak: 'break-word' as const,
    color: '#dc2626',
  },
  actions: {
    display: 'flex',
    gap: '0.5rem',
    justifyContent: 'center',
    flexWrap: 'wrap' as const,
  },
  primaryButton: {
    padding: '0.5rem 1rem',
    backgroundColor: '#3b82f6',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '0.875rem',
    fontWeight: '500',
  },
  secondaryButton: {
    padding: '0.5rem 1rem',
    backgroundColor: 'white',
    color: '#6b7280',
    border: '1px solid #d1d5db',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '0.875rem',
    fontWeight: '500',
  },
};

export default GlobalErrorBoundary;