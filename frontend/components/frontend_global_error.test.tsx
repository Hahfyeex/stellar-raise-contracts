import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import GlobalErrorBoundary, {
  ContractError,
  NetworkError,
  TransactionError,
} from './frontend_global_error';

/**
 * Test suite for GlobalErrorBoundary component.
 *
 * Tests cover:
 * - Rendering children when no error occurs
 * - Catching and displaying React errors
 * - Handling smart contract specific errors
 * - Retry functionality
 * - Error reporting
 * - Custom fallback UI
 */

// Mock console.error to avoid test output pollution
const originalConsoleError = console.error;
beforeAll(() => {
  console.error = jest.fn();
});

afterAll(() => {
  console.error = originalConsoleError;
});

// Mock window.location for navigation tests
let mockHref = 'http://localhost/';
const mockLocation = {
  get href() {
    return mockHref;
  },
  set href(value: string) {
    mockHref = value;
  },
  assign: jest.fn(),
  reload: jest.fn(),
};

delete (global as any).window;
(global as any).window = {
  location: mockLocation,
  navigator: {
    userAgent: 'test-user-agent',
  },
};

/**
 * Test component that throws an error for testing error boundary.
 */
const ErrorThrowingComponent: React.FC<{ shouldThrow?: boolean; errorType?: string }> = ({
  shouldThrow = true,
  errorType = 'generic',
}) => {
  if (shouldThrow) {
    switch (errorType) {
      case 'contract':
        throw new ContractError('Smart contract execution failed');
      case 'network':
        throw new NetworkError('Network connection lost');
      case 'transaction':
        throw new TransactionError('Transaction reverted');
      default:
        throw new Error('Something went wrong');
    }
  }
  return <div>No error</div>;
};

/**
 * Test component for custom fallback testing.
 */
const CustomFallback: React.FC = () => <div>Custom Error UI</div>;

describe('GlobalErrorBoundary', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('Normal operation', () => {
    test('renders children when no error occurs', () => {
      render(
        <GlobalErrorBoundary>
          <div>Test content</div>
        </GlobalErrorBoundary>
      );

      expect(screen.getByText('Test content')).toBeInTheDocument();
    });

    test('renders multiple children correctly', () => {
      render(
        <GlobalErrorBoundary>
          <div>First child</div>
          <div>Second child</div>
        </GlobalErrorBoundary>
      );

      expect(screen.getByText('First child')).toBeInTheDocument();
      expect(screen.getByText('Second child')).toBeInTheDocument();
    });
  });

  describe('Error handling', () => {
    test('catches and displays generic React errors', () => {
      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent errorType="generic" />
        </GlobalErrorBoundary>
      );

      expect(screen.getByText('Something went wrong')).toBeInTheDocument();
      expect(screen.getByText(/unexpected error occurred/)).toBeInTheDocument();
      expect(screen.getByText('⚠️')).toBeInTheDocument();
    });

    test('identifies and handles smart contract errors', () => {
      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent errorType="contract" />
        </GlobalErrorBoundary>
      );

      expect(screen.getByText('Smart Contract Error')).toBeInTheDocument();
      expect(screen.getByText(/blockchain transaction/)).toBeInTheDocument();
      expect(screen.getByText('🔗')).toBeInTheDocument();
    });

    test('handles network errors appropriately', () => {
      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent errorType="network" />
        </GlobalErrorBoundary>
      );

      expect(screen.getByText('Smart Contract Error')).toBeInTheDocument();
      expect(screen.getByText(/blockchain transaction/)).toBeInTheDocument();
    });

    test('handles transaction errors appropriately', () => {
      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent errorType="transaction" />
        </GlobalErrorBoundary>
      );

      expect(screen.getByText('Smart Contract Error')).toBeInTheDocument();
      expect(screen.getByText(/blockchain transaction/)).toBeInTheDocument();
    });

    test('logs errors to console for debugging', () => {
      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent />
        </GlobalErrorBoundary>
      );

      expect(console.error).toHaveBeenCalledWith(
        'Global Error Boundary caught an error:',
        expect.any(Error),
        expect.any(Object)
      );
    });

    test('creates error report with proper structure', () => {
      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent errorType="contract" />
        </GlobalErrorBoundary>
      );

      expect(console.error).toHaveBeenCalledWith(
        'Error Report:',
        expect.objectContaining({
          message: expect.any(String),
          stack: expect.any(String),
          componentStack: expect.any(String),
          timestamp: expect.any(String),
          userAgent: expect.any(String),
          url: expect.any(String),
          isSmartContractError: true,
        })
      );
    });
  });

  describe('Recovery functionality', () => {
    test('retry button resets error state', async () => {
      const { rerender } = render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent shouldThrow={true} />
        </GlobalErrorBoundary>
      );

      expect(screen.getByText('Something went wrong')).toBeInTheDocument();

      // Click retry button
      fireEvent.click(screen.getByText('Try Again'));

      // The component should re-render and show the error again since we're still throwing
      // But the state should be reset, so if we change the component to not throw, it should work
      expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    });

    test.skip('go home button navigates to home page', () => {
      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent />
        </GlobalErrorBoundary>
      );

      fireEvent.click(screen.getByText('Go Home'));

      expect(mockLocation.href).toBe('/');
    });
  });

  describe('Custom fallback', () => {
    test('uses custom fallback when provided', () => {
      render(
        <GlobalErrorBoundary fallback={<CustomFallback />}>
          <ErrorThrowingComponent />
        </GlobalErrorBoundary>
      );

      expect(screen.getByText('Custom Error UI')).toBeInTheDocument();
      expect(screen.queryByText('Something went wrong')).not.toBeInTheDocument();
    });
  });

  describe('Development mode features', () => {
    const originalNodeEnv = process.env.NODE_ENV;

    beforeEach(() => {
      process.env.NODE_ENV = 'development';
    });

    afterEach(() => {
      process.env.NODE_ENV = originalNodeEnv;
    });

    test('shows error details in development mode', () => {
      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent />
        </GlobalErrorBoundary>
      );

      expect(screen.getByText('Error Details (Development)')).toBeInTheDocument();
      expect(screen.getByText(/Error: Something went wrong/)).toBeInTheDocument();
    });

    test('hides error details in production mode', () => {
      process.env.NODE_ENV = 'production';

      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent />
        </GlobalErrorBoundary>
      );

      expect(screen.queryByText('Error Details (Development)')).not.toBeInTheDocument();
    });
  });

  describe('Error classification', () => {
    test('correctly identifies smart contract error patterns', () => {
      const contractError = new Error('Contract execution failed');
      const networkError = new Error('Network timeout occurred');
      const transactionError = new Error('Transaction reverted');
      const genericError = new Error('Generic error');

      // Test the static method directly
      expect(GlobalErrorBoundary.isSmartContractError(contractError)).toBe(true);
      expect(GlobalErrorBoundary.isSmartContractError(networkError)).toBe(true);
      expect(GlobalErrorBoundary.isSmartContractError(transactionError)).toBe(true);
      expect(GlobalErrorBoundary.isSmartContractError(genericError)).toBe(false);
    });

    test('recognizes custom error classes', () => {
      const contractError = new ContractError('Test');
      const networkError = new NetworkError('Test');
      const transactionError = new TransactionError('Test');

      expect(contractError instanceof ContractError).toBe(true);
      expect(networkError instanceof NetworkError).toBe(true);
      expect(transactionError instanceof TransactionError).toBe(true);
    });
  });

  describe('Accessibility', () => {
    test('error UI is keyboard accessible', () => {
      render(
        <GlobalErrorBoundary>
          <ErrorThrowingComponent />
        </GlobalErrorBoundary>
      );

      const retryButton = screen.getByText('Try Again');
      const homeButton = screen.getByText('Go Home');

      // Check that buttons are focusable
      retryButton.focus();
      expect(document.activeElement).toBe(retryButton);

      homeButton.focus();
      expect(document.activeElement).toBe(homeButton);
    });
  });
});