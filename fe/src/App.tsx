import { useEffect, useState } from 'react'
import { Card, Text, Title, Grid, Modal, PasswordInput, Button } from '@mantine/core'
import { useDisclosure } from '@mantine/hooks';
import { useForm } from '@mantine/form';

interface Assignment {
  name: string,
  status: string,
}

interface Report {
  id: number,
  assignments: Assignment[],
}

type ClickInfo = {
  id: number,
  isTitle: boolean,
  assignment_name: string
}

function SubmittedFiles({ files }: { files: String[] }) {
  return (<ul>{
    files.map((file, index) => <li key={index}>{file}</li>)
  }</ul>)
}

function LoginForm() {
  const form = useForm({
    mode: 'uncontrolled',
    initialValues: {
      password: ''
    }
  });

  const handleSubmit = (values: typeof form.values) => {
    fetch(`${import.meta.env.VITE_API_HOST}/login`, {
      method: "post",
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(values)
    }).then(
      (response) => {
        return response.json();
      }
    ).then((response: string) => sessionStorage.setItem("jwt", response));
  }

  return (<form onSubmit={form.onSubmit(handleSubmit)}>
    <PasswordInput placeholder='password' required {...form.getInputProps('password')}>
    </PasswordInput>
    <Button type="submit">Login</Button>
  </form>)
}

function PdfFile({ id, assignment_name }: { id: number, assignment_name: string }) {
  const [content, setContent] = useState<string>("");
  const token = sessionStorage.getItem(("jwt"));
  useEffect(() => {
    if (token) {
      fetch(`${import.meta.env.VITE_API_HOST}/reports/${id}/assignments/${assignment_name}`, {
        headers: {
          'Accept': 'application/json',
          'Authorization': `Bearer ${token}`
        }
      }).then((response) => {
        return response.json()
      }).then((response: string) => {
        const byteCharacters = atob(response);
        const byteNumbers = new Array(byteCharacters.length).fill(0).map((_, i) => byteCharacters.charCodeAt(i));
        const byteArray = new Uint8Array(byteNumbers);
        const blob = new Blob([byteArray], { type: 'application/pdf' });
        const url = URL.createObjectURL(blob);
        setContent(url);
      });
      console.log("PdfFile component called");
    }
  }, []);
  return (token ? <iframe src={content} width="100%" height="800px"></iframe> : <LoginForm></LoginForm>)
}

function App() {

  // report for all students 
  const [data, setData] = useState<Report[]>([]);
  // files for the student that was clicked
  const [files, setFiles] = useState<String[]>([]);
  // the pdf of the file that was clicked
  const [clickInfo, setClickInfo] = useState<ClickInfo>({ id: 0, isTitle: false, assignment_name: "" });

  useEffect(() => {
    fetch(`${import.meta.env.VITE_API_HOST}`).then(
      (response) => {
        return response.json();
      }
    ).then((response: Report[]) => setData(response));
  }, []);

  useEffect(() => {
    fetch(`${import.meta.env.VITE_API_HOST}/files/${clickInfo?.id} `).then(
      (response) => {
        return response.json();
      }
    ).then((response: String[]) => setFiles(response));
  }, [clickInfo]);

  const [opened, { open, close }] = useDisclosure(false);

  return (
    <>
      <Grid gutter={10}>
        {
          data.map((report) =>
            <Grid.Col span={{ base: 12, sm: 4 }} key={report.id}>
              <Card shadow="sm" padding="lg" radius="md" withBorder>
                <Title order={4} onClick={() => { open(); setClickInfo({ id: report.id, isTitle: true, assignment_name: "" }) }}>ID: {report.id}</Title>
                {report.assignments.map(
                  (assignment, index) => (
                    <Text key={index} onClick={() => { open(); setClickInfo({ id: report.id, isTitle: false, assignment_name: assignment.name }) }}>
                      {assignment.name}: {assignment.status}
                    </Text>
                  )
                )}
              </Card>
            </Grid.Col >
          )
        }
      </Grid >
      <Modal opened={opened} onClose={close} fullScreen title={clickInfo?.id} centered>
        {
          clickInfo.isTitle ?
            <SubmittedFiles files={files}></SubmittedFiles> :
            <PdfFile id={clickInfo.id} assignment_name={clickInfo.assignment_name}></PdfFile>
        }
      </Modal>
    </>
  )
}

export default App
