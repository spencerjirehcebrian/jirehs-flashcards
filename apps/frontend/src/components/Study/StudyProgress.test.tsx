import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StudyProgress } from './StudyProgress';

describe('StudyProgress', () => {
  it('should render progress text with current and total', () => {
    render(<StudyProgress current={5} total={10} />);

    expect(screen.getByText('5 / 10')).toBeInTheDocument();
  });

  it('should calculate correct percentage for progress bar', () => {
    const { container } = render(<StudyProgress current={5} total={10} />);

    const progressFill = container.querySelector('.progress-fill');
    expect(progressFill).toHaveStyle({ width: '50%' });
  });

  it('should handle 0% progress', () => {
    const { container } = render(<StudyProgress current={0} total={10} />);

    const progressFill = container.querySelector('.progress-fill');
    expect(progressFill).toHaveStyle({ width: '0%' });
    expect(screen.getByText('0 / 10')).toBeInTheDocument();
  });

  it('should handle 100% progress', () => {
    const { container } = render(<StudyProgress current={10} total={10} />);

    const progressFill = container.querySelector('.progress-fill');
    expect(progressFill).toHaveStyle({ width: '100%' });
    expect(screen.getByText('10 / 10')).toBeInTheDocument();
  });

  it('should handle edge case when total is 0', () => {
    const { container } = render(<StudyProgress current={0} total={0} />);

    const progressFill = container.querySelector('.progress-fill');
    expect(progressFill).toHaveStyle({ width: '0%' });
    expect(screen.getByText('0 / 0')).toBeInTheDocument();
  });

  it('should render with correct class names', () => {
    const { container } = render(<StudyProgress current={3} total={10} />);

    expect(container.querySelector('.study-progress')).toBeInTheDocument();
    expect(container.querySelector('.progress-bar')).toBeInTheDocument();
    expect(container.querySelector('.progress-fill')).toBeInTheDocument();
    expect(container.querySelector('.progress-text')).toBeInTheDocument();
  });
});
