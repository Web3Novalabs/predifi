import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Input } from '../components/ui/Input';

describe('Input', () => {
  it('renders with a placeholder', () => {
    render(<Input placeholder="Enter your name" />);
    expect(screen.getByPlaceholderText('Enter your name')).toBeInTheDocument();
  });

  it('forwards its ref to the underlying input element', () => {
    const ref = React.createRef<HTMLInputElement>();
    render(<Input ref={ref} />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });

  it('calls onChange when the user types', async () => {
    const handleChange = jest.fn();
    render(<Input onChange={handleChange} />);
    const input = screen.getByRole('textbox');
    await userEvent.type(input, 'hello');
    expect(handleChange).toHaveBeenCalledTimes(5);
  });

  it('respects disabled', () => {
    render(<Input disabled placeholder="Disabled input" />);
    expect(screen.getByPlaceholderText('Disabled input')).toBeDisabled();
  });

  it('keeps password visibility toggle keyboard-accessible', async () => {
    const user = userEvent.setup();
    render(<Input type="password" label="Password" />);

    const input = screen.getByLabelText('Password');
    await user.tab();
    expect(input).toHaveFocus();

    await user.tab();
    const toggle = screen.getByRole('button', { name: 'Show password' });
    expect(toggle).toHaveFocus();
    expect(toggle).toHaveAttribute('aria-pressed', 'false');

    fireEvent.click(toggle);
    expect(input).toHaveAttribute('type', 'text');
    expect(screen.getByRole('button', { name: 'Hide password' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
  });
});
