import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TwoPointRatingButtons } from './TwoPointRatingButtons';

describe('TwoPointRatingButtons', () => {
  it('should render Wrong and Correct buttons', () => {
    const onRate = vi.fn();

    render(<TwoPointRatingButtons onRate={onRate} />);

    expect(screen.getByRole('button', { name: 'Wrong' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Correct' })).toBeInTheDocument();
  });

  it('should call onRate with 1 when Wrong is clicked', async () => {
    const user = userEvent.setup();
    const onRate = vi.fn();

    render(<TwoPointRatingButtons onRate={onRate} />);

    await user.click(screen.getByRole('button', { name: 'Wrong' }));

    expect(onRate).toHaveBeenCalledWith(1);
  });

  it('should call onRate with 3 when Correct is clicked', async () => {
    const user = userEvent.setup();
    const onRate = vi.fn();

    render(<TwoPointRatingButtons onRate={onRate} />);

    await user.click(screen.getByRole('button', { name: 'Correct' }));

    expect(onRate).toHaveBeenCalledWith(3);
  });

  it('should disable buttons when disabled prop is true', () => {
    const onRate = vi.fn();

    render(<TwoPointRatingButtons onRate={onRate} disabled />);

    expect(screen.getByRole('button', { name: 'Wrong' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Correct' })).toBeDisabled();
  });

  it('should not call onRate when disabled', async () => {
    const user = userEvent.setup();
    const onRate = vi.fn();

    render(<TwoPointRatingButtons onRate={onRate} disabled />);

    await user.click(screen.getByRole('button', { name: 'Correct' }));

    expect(onRate).not.toHaveBeenCalled();
  });

  it('should default disabled to false', () => {
    const onRate = vi.fn();

    render(<TwoPointRatingButtons onRate={onRate} />);

    expect(screen.getByRole('button', { name: 'Wrong' })).not.toBeDisabled();
    expect(screen.getByRole('button', { name: 'Correct' })).not.toBeDisabled();
  });

  it('should render with correct class names', () => {
    const onRate = vi.fn();

    const { container } = render(<TwoPointRatingButtons onRate={onRate} />);

    expect(container.querySelector('.rating-buttons.two-point')).toBeInTheDocument();
    expect(container.querySelector('.rating-wrong')).toBeInTheDocument();
    expect(container.querySelector('.rating-correct')).toBeInTheDocument();
  });
});
