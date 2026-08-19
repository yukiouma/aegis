import { createTheme } from '@mui/material/styles';

export const ntdTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#FF007F', // Vivid Neon Pink/Magenta
      light: '#FF54A7',
      dark: '#B30059',
      contrastText: '#FFFFFF',
    },
    secondary: {
      main: '#1DE9B6', // Cyan/Teal accents seen in the background
    },
    background: {
      default: '#0A0005', // Extremely dark burgundy/black
      paper: 'rgba(40, 0, 20, 0.7)', // Semi-transparent "Glass" panels
    },
    text: {
      primary: '#FFCCE6', // Light pinkish-white for readability
      secondary: '#FF007F',
    },
    divider: 'rgba(255, 0, 127, 0.3)',
  },
  // typography: {
  //   fontFamily: '"Orbitron", "Roboto Mono", monospace',
  //   h1: { fontWeight: 900, letterSpacing: '0.2em' },
  //   button: { textTransform: 'uppercase', fontWeight: 700, letterSpacing: '0.1em' },
  // },
  shape: {
    borderRadius: 2, // Sharp, technical edges
  },
  components: {
    MuiCssBaseline: {
      styleOverrides: {
        body: {
          // Subtle CRT/Scanline effect
          backgroundImage: `linear-gradient(rgba(18, 16, 16, 0) 50%, rgba(0, 0, 0, 0.2) 50%),
                            linear-gradient(90deg, rgba(255, 0, 0, 0.05), rgba(0, 255, 0, 0.02), rgba(0, 0, 255, 0.05))`,
          backgroundSize: '100% 4px, 4px 100%',
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: 'none',
          backdropFilter: 'blur(10px)', // Glassmorphism
          border: '1px solid rgba(255, 0, 127, 0.4)',
          boxShadow: '0 0 15px rgba(255, 0, 127, 0.2)',
        },
      },
    },
    MuiButton: {
      styleOverrides: {
        root: {
          transition: 'all 0.2s ease-in-out',
          '&:hover': {
            boxShadow: '0 0 20px rgba(255, 0, 127, 0.6)',
            backgroundColor: 'rgba(255, 0, 127, 0.1)',
          },
        },
        // containedPrimary: {
        //   boxShadow: '0 0 10px rgba(255, 0, 127, 0.5)',
        // },
      },
    },
  },
});
