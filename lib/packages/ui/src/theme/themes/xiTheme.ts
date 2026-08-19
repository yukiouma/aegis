import { createTheme } from '@mui/material/styles';

export const xiTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#8A99E8', // The glowing lavender/blue
      contrastText: '#02020A',
    },
    secondary: {
      main: '#3E4675', // Muted tactical blue
    },
    background: {
      default: '#02020A', // Deep space black
      paper: '#0B0B24',   // Navy cockpit panels
    },
    text: {
      primary: '#c4caf0',   // High-readability white/blue
      secondary: '#E0E4FF', // Dimmer HUD text
    },
    divider: 'rgba(138, 153, 232, 0.2)',
  },
  // typography: {
  //   fontFamily: '"Orbitron", "Roboto Mono", monospace',
  //   h1: { fontWeight: 900, letterSpacing: '0.2em' },
  //   button: { textTransform: 'uppercase', fontWeight: 700, letterSpacing: '0.1em' },
  // },
  shape: {
    borderRadius: 0, // This is critical for the tactical/military look
  },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          borderWidth: '2px',
          '&:hover': {
            borderWidth: '2px',
            boxShadow: '0 0 15px rgba(138, 153, 232, 0.4)',
          },
        },
        contained: {
          boxShadow: 'none',
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: 'none', // Removes MUI's default elevation overlay
          border: '1px solid rgba(138, 153, 232, 0.3)',
          // Adding a subtle scanline effect to all Paper components
          background: 'linear-gradient(rgba(11, 11, 36, 0.9), rgba(11, 11, 36, 0.9)), repeating-linear-gradient(0deg, transparent, transparent 1px, rgba(0, 0, 0, 0.1) 1px, rgba(0, 0, 0, 0.1) 2px)',
          boxShadow: '0 0 15px rgba(81, 0, 255, 0.33)',
        },
      },
    },
    MuiOutlinedInput: {
      styleOverrides: {
        root: {
          '& .MuiOutlinedInput-notchedOutline': {
            borderColor: 'rgba(138, 153, 232, 0.4)',
          },
          '&:hover .MuiOutlinedInput-notchedOutline': {
            borderColor: '#8A99E8',
          },
        },
      },
    },
    MuiCssBaseline: {
      styleOverrides: `
        @keyframes pulse {
          0% { opacity: 0.8; }
          50% { opacity: 1; }
          100% { opacity: 0.8; }
        }
        body {
          background-attachment: fixed;
        }
      `,
    },
  },
});
